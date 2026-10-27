use crate::core::services::{unit_step, ServiceKind, ServicesState, DEMOLISH_REFUND};
use crate::core::settings::{Currency, Settings};
use crate::core::{BuildError, Simulation};
use crate::i18n::{I18n, Lang};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Resumen,
    Servicios,
    Economia,
    Energia,
    Informes,
}

/// Ventanas modales del menú de partida.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modal {
    Restart,
    Save,
    Load,
    Settings,
}

/// Resultado de la última orden del jugador, traducido al dibujar
/// (así el aviso sigue al idioma activo si se cambia en Configuración).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Notice {
    Built { kind: ServiceKind, units: f32, cost: f64 },
    Demolished { kind: ServiceKind, units: f32, refund: f64 },
    Failed(BuildError),
}

pub struct MacroSimApp {
    pub sim: Simulation,
    pub settings: Settings,
    pub i18n: I18n,
    pub tab: Tab,
    pub modal: Option<Modal>,
    notice: Option<Notice>,
}

impl Default for MacroSimApp {
    fn default() -> Self {
        Self {
            sim: Simulation::default(),
            settings: Settings::default(),
            i18n: I18n::new(Lang::default()),
            tab: Tab::Resumen,
            modal: None,
            notice: None,
        }
    }
}

impl eframe::App for MacroSimApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dt = ctx.input(|i| i.unstable_dt).min(0.1);
        self.sim.update(dt);
        // Repintar para que la simulación avance en tiempo real
        if !self.sim.paused {
            ctx.request_repaint();
        }
        self.ui(ctx);
    }
}

impl MacroSimApp {
    /// Dibuja toda la interfaz. Separado de `eframe::App::update` para poder
    /// renderizarla en tests headless (sin ventana ni `eframe::Frame`).
    pub fn ui(&mut self, ctx: &egui::Context) {

        // Barra superior: tiempo y controles estilo SimCity / Cities Skylines
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(self.i18n.t("app.title").to_owned());
                ui.separator();
                let date = format!(
                    "{:02}/{:04}",
                    self.sim.clock.month, self.sim.clock.year
                );
                let money = self.settings.currency.format_money(self.sim.economy.money);
                let status = self.i18n.tf(
                    "top.status",
                    &[
                        &date,
                        &money,
                        &self.sim.economy.population.to_string(),
                        &self
                            .settings
                            .currency
                            .format_decimal(self.sim.economy.unemployment_rate as f64, 1),
                    ],
                );
                ui.label(status);
                let land_pct = self.sim.land.usage_percent(&self.sim.services, &self.sim.energy);
                let land_text = self.i18n.tf(
                    "top.land",
                    &[&self.settings.currency.format_decimal(land_pct as f64, 1)],
                );
                let land_color = if land_pct >= 90.0 {
                    egui::Color32::RED
                } else {
                    egui::Color32::from_gray(180)
                };
                ui.colored_label(land_color, land_text);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Menú de partida (botones más a la derecha)
                    if ui.button(self.i18n.t("menu.settings")).clicked() {
                        self.modal = Some(Modal::Settings);
                    }
                    if ui.button(self.i18n.t("menu.restart")).clicked() {
                        self.modal = Some(Modal::Restart);
                    }
                    if ui.button(self.i18n.t("menu.load")).clicked() {
                        self.modal = Some(Modal::Load);
                    }
                    if ui.button(self.i18n.t("menu.save")).clicked() {
                        self.modal = Some(Modal::Save);
                    }
                    ui.separator();
                    let label = if self.sim.paused {
                        self.i18n.t("top.reanudar").to_owned()
                    } else {
                        self.i18n.t("top.pausa").to_owned()
                    };
                    if ui.button(label).clicked() {
                        self.sim.paused = !self.sim.paused;
                    }
                    let speed_text = self.i18n.t("top.speed").to_owned();
                    ui.add(egui::Slider::new(&mut self.sim.speed, 1..=12).text(speed_text));
                    if ui.button(self.i18n.t("top.month_plus")).clicked() {
                        self.sim.tick_month();
                    }
                });
            });
        });

        // Panel lateral: políticas fiscales y gasto (subir/bajar impuestos y gastos)
        // min_width = default_width: evita que al estrecharlo se rompa el layout
        egui::SidePanel::left("dashboard_panel")
            .min_width(300.0)
            .default_width(300.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading(self.i18n.t("panel.state").to_owned());
                ui.separator();

                ui.label(self.i18n.tf(
                    "panel.budget",
                    &[&self.settings.currency.format_money(self.sim.economy.money)],
                ));
                ui.label(self.i18n.tf(
                    "panel.population",
                    &[&self.sim.economy.population.to_string()],
                ));
                ui.label(self.i18n.tf(
                    "panel.unemployment",
                    &[&self
                        .settings
                        .currency
                        .format_decimal(self.sim.economy.unemployment_rate as f64, 1)],
                ));

                let used_ha = self.sim.land.used(&self.sim.services, &self.sim.energy);
                let total_ha = self.sim.land.total_hectareas;
                let land_pct = self.sim.land.usage_percent(&self.sim.services, &self.sim.energy);
                let land_text = self.i18n.tf(
                    "panel.land",
                    &[
                        &self.settings.currency.format_decimal(used_ha as f64, 0),
                        &self.settings.currency.format_decimal(total_ha as f64, 0),
                        &self.settings.currency.format_decimal(land_pct as f64, 1),
                    ],
                );
                if land_pct >= 90.0 {
                    ui.colored_label(egui::Color32::RED, land_text);
                } else {
                    ui.label(land_text);
                }

                // Saldo migratorio del último mes: explica el cambio de población.
                let migration = self.sim.population.net_rate * 100.0;
                let migration_text = self.i18n.tf(
                    "panel.migration",
                    &[&self
                        .settings
                        .currency
                        .format_decimal(migration as f64, 2)],
                );
                if migration > 0.0 {
                    ui.colored_label(egui::Color32::GREEN, migration_text);
                } else if migration < 0.0 {
                    ui.colored_label(egui::Color32::RED, migration_text);
                } else {
                    ui.label(migration_text);
                }

                ui.add_space(10.0);
                ui.heading(self.i18n.t("panel.energy").to_owned());
                ui.label(self.i18n.tf(
                    "panel.capacity",
                    &[&self
                        .settings
                        .currency
                        .format_decimal(self.sim.energy.capacity_mw as f64, 0)],
                ));
                ui.label(self.i18n.tf(
                    "panel.demand",
                    &[&self
                        .settings
                        .currency
                        .format_decimal(self.sim.energy.demand_mw as f64, 0)],
                ));
                ui.label(self.i18n.tf(
                    "panel.price",
                    &[&self
                        .settings
                        .currency
                        .format_decimal(self.sim.energy.price_per_mwh as f64, 1)],
                ));
                let capacity_text = self.i18n.t("panel.capacity_slider").to_owned();
                ui.add(
                    egui::Slider::new(&mut self.sim.energy.capacity_mw, 0.0..=2000.0)
                        .text(capacity_text),
                );

                ui.add_space(15.0);
                ui.heading(self.i18n.t("panel.policies").to_owned());
                let corp_text = self.i18n.t("panel.corporate_tax").to_owned();
                ui.add(
                    egui::Slider::new(&mut self.sim.economy.corporate_tax, 0.0..=100.0)
                        .text(corp_text),
                );
                // IRPF por tramos: cada clase tiene su propio tipo.
                let low_text = self.i18n.t("panel.income_tax_low").to_owned();
                ui.add(
                    egui::Slider::new(&mut self.sim.economy.income_tax_low, 0.0..=100.0)
                        .text(low_text),
                );
                let mid_text = self.i18n.t("panel.income_tax_mid").to_owned();
                ui.add(
                    egui::Slider::new(&mut self.sim.economy.income_tax_mid, 0.0..=100.0)
                        .text(mid_text),
                );
                let high_text = self.i18n.t("panel.income_tax_high").to_owned();
                ui.add(
                    egui::Slider::new(&mut self.sim.economy.income_tax_high, 0.0..=100.0)
                        .text(high_text),
                );
                let spend_text = self.i18n.tf("panel.spending", &[&self.settings.currency.symbol]);
                ui.add(
                    egui::Slider::new(&mut self.sim.economy.public_spending, 0.0..=20.0)
                        .text(spend_text),
                );

                let cur = &self.settings.currency;
                let effective = self
                    .sim
                    .economy
                    .effective_tax_rate(&self.sim.demographics);
                ui.separator();
                ui.label(self.i18n.tf(
                    "panel.tax_pressure",
                    &[&cur.format_decimal(effective as f64, 1)],
                ));
                let flight = self.sim.economy.capital_flight();
                if flight > 0.0 {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        self.i18n.tf(
                            "panel.capital_flight",
                            &[&cur.format_decimal(flight as f64 * 100.0, 0)],
                        ),
                    );
                }

                let balance = self
                    .sim
                    .economy
                    .monthly_balance(&self.sim.demographics);
                ui.separator();
                ui.label(self.i18n.tf(
                    "panel.balance",
                    &[&cur.format_money(balance)],
                ));
                if balance < 0.0 {
                    ui.colored_label(egui::Color32::RED, self.i18n.t("panel.deficit"));
                } else {
                    ui.colored_label(egui::Color32::GREEN, self.i18n.t("panel.surplus"));
                }
            });

        // Panel central: informes por pestañas
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                let resumen = self.i18n.t("tab.summary").to_owned();
                let servicios = self.i18n.t("tab.services").to_owned();
                let economia = self.i18n.t("tab.economy").to_owned();
                let energia = self.i18n.t("tab.energy").to_owned();
                let informes = self.i18n.t("tab.reports").to_owned();
                ui.selectable_value(&mut self.tab, Tab::Resumen, resumen);
                ui.selectable_value(&mut self.tab, Tab::Servicios, servicios);
                ui.selectable_value(&mut self.tab, Tab::Economia, economia);
                ui.selectable_value(&mut self.tab, Tab::Energia, energia);
                ui.selectable_value(&mut self.tab, Tab::Informes, informes);
            });
            ui.separator();

            let no_data = self.i18n.t("plot.no_data").to_owned();
            match self.tab {
                Tab::Resumen => {
                    let outlook = self.sim.population.outlook(
                        &self.sim.demographics,
                        &self.sim.services,
                        &self.sim.economy,
                    );
                    let cur = &self.settings.currency;
                    ui.heading(self.i18n.t("summary.population"));
                    plot_generic(ui, &self.sim.history.population, &no_data);
                    ui.label(self.i18n.tf(
                        "summary.satisfaction",
                        &[&cur.format_decimal(outlook.satisfaction as f64 * 100.0, 0)],
                    ));
                    ui.label(self.i18n.tf(
                        "summary.migration_detail",
                        &[
                            &cur.format_decimal(outlook.net_rate as f64 * 100.0, 2),
                            &cur.format_decimal(outlook.from_services as f64 * 100.0, 2),
                            &cur.format_decimal(outlook.from_jobs as f64 * 100.0, 2),
                            &cur.format_decimal(outlook.from_taxes as f64 * 100.0, 2),
                        ],
                    ));
                    ui.add_space(8.0);
                    ui.heading(self.i18n.t("summary.budget_evolution"));
                    plot_money(ui, &self.sim.history.money, &no_data);
                    ui.add_space(8.0);
                    ui.heading(self.i18n.t("summary.unemployment"));
                    plot_generic(ui, &self.sim.history.unemployment, &no_data);
                }
                Tab::Servicios => self.show_services(ui),
                Tab::Economia => {
                    let econ = &self.sim.economy;
                    let demo = &self.sim.demographics;
                    let cur = &self.settings.currency;
                    let income = econ.monthly_income(demo);
                    let expenses = econ.monthly_expenses();
                    let upkeep = econ.service_upkeep;
                    let spending = expenses - upkeep;
                    let irpf = econ.irpf_per_capita(demo) as f64 * econ.population as f64;
                    let corporate = econ.monthly_income(demo) - irpf;
                    let effective = econ.effective_tax_rate(demo);
                    let flight = econ.capital_flight();

                    ui.heading(self.i18n.t("economy.report"));
                    ui.label(self.i18n.tf(
                        "economy.incomes_note",
                        &[
                            &cur.format_decimal(econ.income_tax_low as f64, 1),
                            &cur.format_decimal(econ.income_tax_mid as f64, 1),
                            &cur.format_decimal(econ.income_tax_high as f64, 1),
                        ],
                    ));
                    ui.add_space(6.0);
                    ui.label(self.i18n.tf(
                        "economy.income_line",
                        &[
                            &cur.format_money(income),
                            &cur.format_money(irpf),
                            &cur.format_money(corporate),
                        ],
                    ));
                    ui.label(self.i18n.tf(
                        "economy.expense_line",
                        &[
                            &cur.format_money(expenses),
                            &cur.format_money(spending),
                            &cur.format_money(upkeep),
                        ],
                    ));
                    ui.label(self.i18n.tf(
                        "economy.effective_rate",
                        &[&cur.format_decimal(effective as f64, 1)],
                    ));
                    if flight > 0.0 {
                        ui.colored_label(
                            egui::Color32::YELLOW,
                            self.i18n.tf(
                                "economy.capital_flight_line",
                                &[&cur.format_decimal(flight as f64 * 100.0, 0)],
                            ),
                        );
                    }
                    ui.add_space(8.0);
                    ui.heading(self.i18n.t("economy.budget_evolution"));
                    plot_money(ui, &self.sim.history.money, &no_data);
                    ui.heading(self.i18n.t("summary.unemployment"));
                    plot_generic(ui, &self.sim.history.unemployment, &no_data);
                }
                Tab::Energia => {
                    ui.heading(self.i18n.t("energy.report"));
                    let coverage = self.sim.energy.coverage();
                    ui.label(self.i18n.tf(
                        "energy.coverage",
                        &[&self
                            .settings
                            .currency
                            .format_decimal(coverage as f64, 1)],
                    ));
                    if self.sim.energy.demand_mw > self.sim.energy.capacity_mw {
                        ui.colored_label(
                            egui::Color32::RED,
                            self.i18n.t("energy.deficit_warn"),
                        );
                    }
                    plot_generic(ui, &self.sim.history.demand, &no_data);
                }
                Tab::Informes => {
                    ui.heading(self.i18n.t("reports.title"));
                    egui::Grid::new("informe_grid")
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(self.i18n.t("reports.month_col"));
                            ui.label(self.i18n.t("reports.pop_col"));
                            ui.label(self.i18n.t("reports.budget_col"));
                            ui.label(self.i18n.t("reports.unemp_col"));
                            ui.end_row();
                            let n = self.sim.history.money.len().min(12);
                            for i in 0..n {
                                let idx = self.sim.history.money.len() - n + i;
                                ui.label(self.i18n.tf(
                                    "reports.months_ago",
                                    &[&(n - i).to_string()],
                                ));
                                let pop = self
                                    .sim
                                    .history
                                    .population
                                    .get(idx)
                                    .copied()
                                    .unwrap_or(0.0);
                                ui.label(self.settings.currency.format_decimal(pop, 0));
                                let money = self.sim.history.money.get(idx).copied().unwrap_or(0.0);
                                ui.label(self.settings.currency.format_money(money));
                                let unemp =
                                    self.sim.history.unemployment.get(idx).copied().unwrap_or(0.0);
                                ui.label(self.settings.currency.format_decimal(unemp, 1));
                                ui.end_row();
                            }
                        });
                }
            }
        });

        self.show_modals(ctx);
    }
}

impl MacroSimApp {
    /// Pestaña de servicios: capacidad pública/privada, cobertura, mantenimiento
    /// y suelo, con botones para construir y demoler. Las órdenes se recogen
    /// mientras se dibuja la tabla y se aplican al terminar (así no se muta el
    /// estado que se está leyendo).
    fn show_services(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.i18n.t("services.title"));
        ui.label(self.i18n.t("services.hint"));
        ui.add_space(6.0);

        let pop = self.sim.economy.population;
        let mut pending: Option<(ServiceKind, f32)> = None;

        egui::Grid::new("services_grid")
            .striped(true)
            .num_columns(8)
            .spacing([14.0, 6.0])
            .show(ui, |ui| {
                ui.strong(self.i18n.t("services.col.service"));
                ui.strong(self.i18n.t("services.col.demand"));
                ui.strong(self.i18n.t("services.col.public"));
                ui.strong(self.i18n.t("services.col.private"));
                ui.strong(self.i18n.t("services.col.coverage"));
                ui.strong(self.i18n.t("services.col.upkeep"));
                ui.strong(self.i18n.t("services.col.land"));
                ui.strong(self.i18n.t("services.col.actions"));
                ui.end_row();

                for kind in ServiceKind::ALL {
                    let spec = kind.spec();
                    let st = self.sim.services.get(kind).clone();
                    let cur = &self.settings.currency;
                    let demand = ServicesState::demand(kind, pop);
                    let step = unit_step(kind, pop);

                    ui.label(self.i18n.t(kind.name_key()));
                    ui.label(cur.format_decimal(demand as f64, 1));
                    ui.label(cur.format_decimal(st.public_units as f64, 1));
                    ui.label(cur.format_decimal(st.private_units as f64, 1));

                    let pct = st.coverage * 100.0;
                    let color = if pct >= 95.0 {
                        egui::Color32::GREEN
                    } else if pct >= 70.0 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::RED
                    };
                    ui.colored_label(
                        color,
                        format!("{}%", cur.format_decimal(pct as f64, 0)),
                    );

                    ui.label(cur.format_money((st.public_units * spec.upkeep_per_unit) as f64));
                    ui.label(cur.format_decimal((st.public_units * spec.land_per_unit) as f64, 1));

                    ui.horizontal(|ui| {
                        let step_text = cur.format_decimal(step as f64, 0);
                        let land_text = cur.format_decimal((step * spec.land_per_unit) as f64, 2);
                        let build_cost = cur.format_money(
                            (step * spec.capital_cost_per_unit) as f64,
                        );
                        let build_tip =
                            self.i18n.tf("services.build_tip", &[&step_text, &build_cost, &land_text]);
                        if ui.button("+").on_hover_text(build_tip).clicked() {
                            pending = Some((kind, step));
                        }
                        let refund = cur.format_money(
                            (step * spec.capital_cost_per_unit * DEMOLISH_REFUND) as f64,
                        );
                        let demolish_tip =
                            self.i18n.tf("services.demolish_tip", &[&step_text, &refund, &land_text]);
                        if ui.button("-").on_hover_text(demolish_tip).clicked() {
                            pending = Some((kind, -step));
                        }
                    });
                    ui.end_row();
                }
            });

        if let Some((kind, delta)) = pending {
            let outcome = if delta > 0.0 {
                self.sim
                    .build(kind, delta)
                    .map(|cost| Notice::Built { kind, units: delta, cost })
            } else {
                self.sim
                    .demolish(kind, -delta)
                    .map(|refund| Notice::Demolished { kind, units: -delta, refund })
            };
            self.notice = Some(outcome.unwrap_or_else(Notice::Failed));
            ui.ctx().request_repaint();
        }

        if let Some(notice) = self.notice {
            ui.add_space(6.0);
            let (text, color) = self.describe_notice(notice);
            ui.colored_label(color, text);
        }

        if self.sim.land.usage_percent(&self.sim.services, &self.sim.energy) >= 90.0 {
            ui.add_space(6.0);
            ui.colored_label(
                egui::Color32::RED,
                self.i18n.t("services.land_warning"),
            );
        }
    }

    /// Traduce el resultado de la última orden al idioma activo.
    fn describe_notice(&self, notice: Notice) -> (String, egui::Color32) {
        let cur = &self.settings.currency;
        match notice {
            Notice::Built { kind, units, cost } => (
                self.i18n.tf(
                    "services.notice.built",
                    &[
                        self.i18n.t(kind.name_key()),
                        &cur.format_decimal(units as f64, 0),
                        &cur.format_money(cost),
                    ],
                ),
                egui::Color32::GREEN,
            ),
            Notice::Demolished { kind, units, refund } => (
                self.i18n.tf(
                    "services.notice.demolished",
                    &[
                        self.i18n.t(kind.name_key()),
                        &cur.format_decimal(units as f64, 0),
                        &cur.format_money(refund),
                    ],
                ),
                egui::Color32::LIGHT_BLUE,
            ),
            Notice::Failed(err) => (
                self.i18n.t(error_key(err)).to_owned(),
                egui::Color32::LIGHT_RED,
            ),
        }
    }

    fn show_modals(&mut self, ctx: &egui::Context) {
        match self.modal {
            Some(Modal::Restart) => self.modal_restart(ctx),
            Some(Modal::Save) => self.modal_save(ctx),
            Some(Modal::Load) => self.modal_load(ctx),
            Some(Modal::Settings) => self.modal_settings(ctx),
            None => {}
        }
    }

    fn modal_restart(&mut self, ctx: &egui::Context) {
        let title = self.i18n.t("modal.restart.title").to_owned();
        let confirm = self.i18n.t("modal.restart.confirm").to_owned();
        let accept = self.i18n.t("modal.restart.accept").to_owned();
        let cancel = self.i18n.t("modal.cancel").to_owned();

        let mut open = true;
        let mut restart = false;
        let mut cancel_clicked = false;
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(confirm);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(accept).clicked() {
                        restart = true;
                    }
                    if ui.button(&cancel).clicked() {
                        cancel_clicked = true;
                    }
                });
            });

        if restart || cancel_clicked {
            if restart {
                self.sim = Simulation::default();
                self.notice = None;
            }
            self.modal = None;
        } else if !open {
            self.modal = None;
        }
    }

    fn modal_save(&mut self, ctx: &egui::Context) {
        let title = self.i18n.t("modal.save.title").to_owned();
        self.info_modal(ctx, &title);
    }

    fn modal_load(&mut self, ctx: &egui::Context) {
        let title = self.i18n.t("modal.load.title").to_owned();
        self.info_modal(ctx, &title);
    }

    /// Ventana informativa genérica (guardar/cargar: próximamente).
    fn info_modal(&mut self, ctx: &egui::Context, title: &str) {
        let body = self.i18n.t("modal.coming_soon").to_owned();
        let mut open = true;
        egui::Window::new(title.to_owned())
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(body);
            });
        if !open {
            self.modal = None;
        }
    }

    fn modal_settings(&mut self, ctx: &egui::Context) {
        let title = self.i18n.t("settings.title").to_owned();
        let lang_label = self.i18n.t("settings.language").to_owned();
        let symbol_label = self.i18n.t("settings.currency_symbol").to_owned();
        let thousands_label = self.i18n.t("settings.thousands_sep").to_owned();
        let decimal_label = self.i18n.t("settings.decimal_sep").to_owned();
        let lang_before = self.settings.lang;

        let mut open = true;
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(lang_label);
                egui::ComboBox::from_id_salt("settings_lang")
                    .selected_text(self.settings.lang.display_name())
                    .show_ui(ui, |ui| {
                        for lang in Lang::ALL {
                            let name = lang.display_name();
                            ui.selectable_value(&mut self.settings.lang, lang, name);
                        }
                    });

                ui.add_space(8.0);
                ui.label(symbol_label);
                egui::ComboBox::from_id_salt("settings_currency_symbol")
                    .selected_text(&self.settings.currency.symbol)
                    .show_ui(ui, |ui| {
                        for symbol in Currency::AVAILABLE_SYMBOLS {
                            ui.selectable_value(
                                &mut self.settings.currency.symbol,
                                symbol.to_owned(),
                                symbol,
                            );
                        }
                    });

                // Los separadores dependen del idioma: visibles pero bloqueados
                ui.add_space(8.0);
                ui.label(thousands_label);
                ui.add_enabled(
                    false,
                    egui::TextEdit::singleline(&mut self.settings.currency.thousands_sep),
                );
                ui.label(decimal_label);
                ui.add_enabled(
                    false,
                    egui::TextEdit::singleline(&mut self.settings.currency.decimal_sep),
                );
            });

        if self.settings.lang != lang_before {
            self.settings.apply_language(self.settings.lang);
            self.i18n.set_lang(self.settings.lang);
        }
        if !open {
            self.modal = None;
        }
    }
}

/// Clave i18n del motivo de fallo (el core no conoce textos).
fn error_key(err: BuildError) -> &'static str {
    match err {
        BuildError::InsufficientFunds => "services.error.no_money",
        BuildError::InsufficientLand => "services.error.no_land",
        BuildError::NoCapacity => "services.error.no_units",
    }
}

fn plot_money(ui: &mut egui::Ui, data: &[f64], no_data: &str) {
    plot_generic(ui, data, no_data);
}

fn plot_generic(ui: &mut egui::Ui, data: &[f64], no_data: &str) {
    use egui::{Color32, Pos2, Sense, Stroke};
    if data.is_empty() {
        ui.label(no_data);
        return;
    }
    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (max - min).max(1e-6);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 180.0), Sense::hover());
    ui.painter().rect_filled(rect, 4.0, Color32::from_gray(24));
    let n = data.len();
    let mut prev: Option<Pos2> = None;
    for (i, v) in data.iter().enumerate() {
        let x = rect.min.x + rect.width() * (i as f32 / (n.max(2) - 1) as f32);
        let t = ((v - min) / range) as f32;
        let y = rect.max.y - 8.0 - t * (rect.height() - 16.0);
        let p = Pos2::new(x, y);
        if let Some(q) = prev {
            ui.painter().line_segment([q, p], Stroke::new(2.0_f32, Color32::LIGHT_BLUE));
        }
        prev = Some(p);
    }
    // Eje min/max
    ui.painter().text(
        Pos2::new(rect.min.x + 4.0, rect.min.y + 4.0),
        egui::Align2::LEFT_TOP,
        format!("max {:.0}", max),
        egui::FontId::proportional(12.0),
        Color32::GRAY,
    );
    ui.painter().text(
        Pos2::new(rect.min.x + 4.0, rect.max.y - 4.0),
        egui::Align2::LEFT_BOTTOM,
        format!("min {:.0}", min),
        egui::FontId::proportional(12.0),
        Color32::GRAY,
    );
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Renderiza la UI sin ventana: detecta pánicos de layout/borrow en todas
    /// las pestañas (incluida la tabla de servicios) sin abrir una ventana.
    #[test]
    fn dibuja_todas_las_pestanas_sin_panicos() {
        let ctx = egui::Context::default();
        let mut app = MacroSimApp::default();
        let tabs = [
            Tab::Resumen,
            Tab::Servicios,
            Tab::Economia,
            Tab::Energia,
            Tab::Informes,
        ];
        for tab in tabs {
            app.tab = tab;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 720.0),
                )),
                ..Default::default()
            };
            let output = ctx.run(input, |ctx| app.ui(ctx));
            assert!(!output.shapes.is_empty(), "la pestaña {tab:?} no dibujó nada");
        }
    }

    /// Toda clave usada por la UI de servicios debe existir en es y en.
    #[test]
    fn las_claves_i18n_de_servicios_existen_en_todos_los_idiomas() {
        let claves = [
            "top.land",
            "panel.land",
            "panel.migration",
            "panel.income_tax_low",
            "panel.income_tax_mid",
            "panel.income_tax_high",
            "panel.tax_pressure",
            "panel.capital_flight",
            "economy.incomes_note",
            "economy.income_line",
            "economy.expense_line",
            "economy.effective_rate",
            "economy.capital_flight_line",
            "economy.budget_evolution",
            "summary.population",
            "summary.satisfaction",
            "summary.migration_detail",
            "reports.pop_col",
            "tab.services",
            "services.title",
            "services.hint",
            "services.col.service",
            "services.col.demand",
            "services.col.public",
            "services.col.private",
            "services.col.coverage",
            "services.col.upkeep",
            "services.col.land",
            "services.col.actions",
            "services.build_tip",
            "services.demolish_tip",
            "services.notice.built",
            "services.notice.demolished",
            "services.land_warning",
        ];
        for lang in Lang::ALL {
            let i18n = I18n::new(lang);
            for key in claves {
                assert_ne!(i18n.t(key), key, "falta la clave '{key}' en {lang:?}");
            }
            for kind in ServiceKind::ALL {
                let key = kind.name_key();
                assert_ne!(i18n.t(key), key, "falta el nombre de {kind:?} en {lang:?}");
            }
            for err in [
                BuildError::InsufficientFunds,
                BuildError::InsufficientLand,
                BuildError::NoCapacity,
            ] {
                let key = error_key(err);
                assert_ne!(i18n.t(key), key, "falta el error {err:?} en {lang:?}");
            }
        }
    }
}

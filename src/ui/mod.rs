mod theme;

use theme::HandCursor;

use crate::core::services::{unit_step, ServiceKind, ServicesState, DEMOLISH_REFUND};
use crate::core::settings::{Currency, Settings, Theme};
use crate::core::{ActionError, EnergySource, Simulation};
use crate::i18n::{I18n, Lang};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Resumen,
    Servicios,
    Economia,
    Energia,
    Informes,
}

/// Sub-pestañas dentro de la pestaña Energía: la tabla de lo instalado, el
/// catálogo de construcción y la gráfica de demanda no caben juntas en una
/// ventana normal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyTab {
    Instaladas,
    Construir,
    Demanda,
}

impl EnergyTab {
    pub const ALL: [EnergyTab; 3] = [
        EnergyTab::Instaladas,
        EnergyTab::Construir,
        EnergyTab::Demanda,
    ];

    pub fn name_key(self) -> &'static str {
        match self {
            EnergyTab::Instaladas => "energy.tab.installed",
            EnergyTab::Construir => "energy.tab.build",
            EnergyTab::Demanda => "energy.tab.demand",
        }
    }
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
    Borrowed { amount: f64 },
    Repaid { amount: f64 },
    PowerBuilt { source: EnergySource, cost: f64 },
    PowerDemolished { source: EnergySource, refund: f64 },
    Failed(ActionError),
}

pub struct MacroSimApp {
    pub sim: Simulation,
    pub settings: Settings,
    pub i18n: I18n,
    pub tab: Tab,
    pub modal: Option<Modal>,
    notice: Option<Notice>,
    /// Importe elegido en el deslizador de deuda (0 = el máximo disponible).
    loan_request: f64,
    /// Sub-pestaña activa dentro de Energía.
    energy_tab: EnergyTab,
    /// Paleta ya aplicada al contexto (evita rehacer el estilo cada frame).
    applied_theme: Option<Theme>,
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
            loan_request: 0.0,
            energy_tab: EnergyTab::Instaladas,
            applied_theme: None,
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
        if self.applied_theme != Some(self.settings.theme) {
            theme::apply(ctx, self.settings.theme);
            self.applied_theme = Some(self.settings.theme);
        }

        self.show_version_bar(ctx);

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
                        &self.settings.currency.format_int(self.sim.economy.population as f64),
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
                    if ui.button(self.i18n.t("menu.settings")).hand(ui).clicked() {
                        self.modal = Some(Modal::Settings);
                    }
                    if ui.button(self.i18n.t("menu.restart")).hand(ui).clicked() {
                        self.modal = Some(Modal::Restart);
                    }
                    if ui.button(self.i18n.t("menu.load")).hand(ui).clicked() {
                        self.modal = Some(Modal::Load);
                    }
                    if ui.button(self.i18n.t("menu.save")).hand(ui).clicked() {
                        self.modal = Some(Modal::Save);
                    }
                    ui.separator();
                    let label = if self.sim.paused {
                        self.i18n.t("top.reanudar").to_owned()
                    } else {
                        self.i18n.t("top.pausa").to_owned()
                    };
                    if ui.button(label).hand(ui).clicked() {
                        self.sim.paused = !self.sim.paused;
                    }
                    let speed_text = self.i18n.t("top.speed").to_owned();
                    ui.add(egui::Slider::new(&mut self.sim.speed, 1..=12).text(speed_text))
                        .hand(ui);
                    if ui.button(self.i18n.t("top.month_plus")).hand(ui).clicked() {
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
                    &[&self
                        .settings
                        .currency
                        .format_int(self.sim.economy.population as f64)],
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
                let efectiva = self.sim.energy.effective_capacity_mw();
                let demanda = self.sim.energy.demand_mw;
                ui.label(self.i18n.tf(
                    "panel.capacity",
                    &[&self.settings.currency.format_decimal(efectiva as f64, 0)],
                ));
                ui.label(self.i18n.tf(
                    "panel.demand",
                    &[&self.settings.currency.format_decimal(demanda as f64, 0)],
                ));
                ui.label(self.i18n.tf(
                    "panel.price",
                    &[&self
                        .settings
                        .currency
                        .format_money(self.sim.energy.cost_per_mwh() as f64)],
                ));
                if self.sim.energy.has_deficit() {
                    ui.colored_label(egui::Color32::RED, self.i18n.t("panel.energy_deficit"));
                }
                ui.label(self.i18n.t("panel.energy_hint"));

                ui.add_space(15.0);
                ui.heading(self.i18n.t("panel.policies").to_owned());
                let corp_text = self.i18n.t("panel.corporate_tax").to_owned();
                ui.add(
                    egui::Slider::new(&mut self.sim.economy.corporate_tax, 0.0..=100.0)
                        .text(corp_text),
                )
                .hand(ui);
                // IRPF por tramos: cada clase tiene su propio tipo.
                let low_text = self.i18n.t("panel.income_tax_low").to_owned();
                ui.add(
                    egui::Slider::new(&mut self.sim.economy.income_tax_low, 0.0..=100.0)
                        .text(low_text),
                )
                .hand(ui);
                let mid_text = self.i18n.t("panel.income_tax_mid").to_owned();
                ui.add(
                    egui::Slider::new(&mut self.sim.economy.income_tax_mid, 0.0..=100.0)
                        .text(mid_text),
                )
                .hand(ui);
                let high_text = self.i18n.t("panel.income_tax_high").to_owned();
                ui.add(
                    egui::Slider::new(&mut self.sim.economy.income_tax_high, 0.0..=100.0)
                        .text(high_text),
                )
                .hand(ui);
                let spend_text = self.i18n.tf("panel.spending", &[&self.settings.currency.symbol]);
                ui.add(
                    egui::Slider::new(&mut self.sim.economy.public_spending, 0.0..=20.0)
                        .text(spend_text),
                )
                .hand(ui);

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

                // Delincuencia: termómetro del gobierno, no un slider.
                let crime = self.sim.crime.rate;
                let crime_text = self.i18n.tf(
                    "panel.crime",
                    &[&cur.format_decimal(crime as f64, 1)],
                );
                let pressure = self.sim.crime.pressure();
                let crime_color = if pressure >= 0.6 {
                    egui::Color32::RED
                } else if pressure >= 0.3 {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::GREEN
                };
                ui.colored_label(crime_color, crime_text);
            });

        // Panel lateral derecho: deuda y préstamos (caja de la ciudad)
        egui::SidePanel::right("debt_panel")
            .min_width(210.0)
            .default_width(210.0)
            .resizable(false)
            .show(ctx, |ui| self.show_debt(ui));

        // Panel central: informes por pestañas
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                let resumen = self.i18n.t("tab.summary").to_owned();
                let servicios = self.i18n.t("tab.services").to_owned();
                let economia = self.i18n.t("tab.economy").to_owned();
                let energia = self.i18n.t("tab.energy").to_owned();
                let informes = self.i18n.t("tab.reports").to_owned();
                ui.selectable_value(&mut self.tab, Tab::Resumen, resumen).hand(ui);
                ui.selectable_value(&mut self.tab, Tab::Servicios, servicios).hand(ui);
                ui.selectable_value(&mut self.tab, Tab::Economia, economia).hand(ui);
                ui.selectable_value(&mut self.tab, Tab::Energia, energia).hand(ui);
                ui.selectable_value(&mut self.tab, Tab::Informes, informes).hand(ui);
            });
            ui.separator();

            let no_data = self.i18n.t("plot.no_data").to_owned();
            match self.tab {
                Tab::Resumen => {
                    let outlook = self.sim.population.outlook(
                        &self.sim.demographics,
                        &self.sim.services,
                        &self.sim.economy,
                        self.sim.crime.pressure(),
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
                            &cur.format_decimal(outlook.from_crime as f64 * 100.0, 2),
                        ],
                    ));
                    ui.add_space(8.0);
                    ui.heading(self.i18n.t("summary.crime"));
                    plot_generic(ui, &self.sim.history.crime, &no_data);
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
                    let expenses = econ.monthly_expenses(demo);
                    let interest = econ.monthly_interest(demo);
                    let upkeep = econ.service_upkeep;
                    let spending = expenses - upkeep - interest;
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
                            &cur.format_money(interest),
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
                    ui.label(self.i18n.tf(
                        "economy.debt_line",
                        &[
                            &cur.format_money(econ.debt),
                            &cur.format_money(econ.credit_limit(demo)),
                        ],
                    ));
                    ui.add_space(8.0);
                    ui.heading(self.i18n.t("economy.budget_evolution"));
                    plot_money(ui, &self.sim.history.money, &no_data);
                    ui.heading(self.i18n.t("summary.debt"));
                    plot_money(ui, &self.sim.history.debt, &no_data);
                    ui.heading(self.i18n.t("summary.unemployment"));
                    plot_generic(ui, &self.sim.history.unemployment, &no_data);
                }
                Tab::Energia => self.show_energy(ui, &no_data),
                Tab::Informes => {
                    ui.heading(self.i18n.t("reports.title"));
                    egui::Grid::new("informe_grid")
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(self.i18n.t("reports.month_col"));
                            ui.label(self.i18n.t("reports.pop_col"));
                            ui.label(self.i18n.t("reports.budget_col"));
                            ui.label(self.i18n.t("reports.debt_col"));
                            ui.label(self.i18n.t("reports.unemp_col"));
                            ui.label(self.i18n.t("reports.crime_col"));
                            ui.end_row();
                            let n = self.sim.history.len().min(12);
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
                                ui.label(self.settings.currency.format_int(pop));
                                let money = self.sim.history.money.get(idx).copied().unwrap_or(0.0);
                                ui.label(self.settings.currency.format_money(money));
                                let debt = self.sim.history.debt.get(idx).copied().unwrap_or(0.0);
                                ui.label(self.settings.currency.format_money(debt));
                                let unemp =
                                    self.sim.history.unemployment.get(idx).copied().unwrap_or(0.0);
                                ui.label(self.settings.currency.format_decimal(unemp, 1));
                                let crime = self.sim.history.crime.get(idx).copied().unwrap_or(0.0);
                                ui.label(self.settings.currency.format_decimal(crime, 1));
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
    /// Esquina inferior derecha con la versión. Solo en compilaciones de
    /// release: en desarrollo no aporta y ocupa sitio.
    fn show_version_bar(&self, ctx: &egui::Context) {
        if cfg!(debug_assertions) {
            return;
        }
        egui::TopBottomPanel::bottom("version_bar")
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(8, 2)))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let text = self.i18n.tf("app.version", &[crate::VERSION]);
                    ui.label(egui::RichText::new(text).small().weak());
                });
            });
    }

    /// Pestaña de energía, en dos tablas:
    /// 1. **Instalaciones actuales**: lo que tienes y lo que te cuesta al mes.
    /// 2. **Construir**: catálogo con el coste y la producción de *una* planta.
    fn show_energy(&mut self, ui: &mut egui::Ui, no_data: &str) {
        // Sub-pestañas: las tres vistas no caben a la vez en una ventana normal.
        ui.horizontal(|ui| {
            for sub in EnergyTab::ALL {
                let label = self.i18n.t(sub.name_key()).to_owned();
                ui.selectable_value(&mut self.energy_tab, sub, label).hand(ui);
            }
        });
        ui.separator();

        self.show_energy_balance(ui);

        let pending = match self.energy_tab {
            EnergyTab::Instaladas => self.show_energy_installed(ui),
            EnergyTab::Construir => self.show_energy_catalog(ui),
            EnergyTab::Demanda => {
                ui.strong(self.i18n.t("energy.demand_chart"));
                ui.add_space(4.0);
                plot_generic(ui, &self.sim.history.demand, no_data);
                None
            }
        };

        if let Some((source, construir)) = pending {
            let outcome = if construir {
                self.sim
                    .build_energy(source)
                    .map(|cost| Notice::PowerBuilt { source, cost })
            } else {
                self.sim
                    .demolish_energy(source)
                    .map(|refund| Notice::PowerDemolished { source, refund })
            };
            self.notice = Some(outcome.unwrap_or_else(Notice::Failed));
            ui.ctx().request_repaint();
        }

        if self.sim.energy.has_deficit() {
            ui.add_space(6.0);
            ui.colored_label(egui::Color32::RED, self.i18n.t("energy.deficit_warn"));
        }
        if let Some(notice) = self.notice {
            ui.add_space(6.0);
            let (text, color) = self.describe_notice(notice);
            ui.colored_label(color, text);
        }
    }

    /// Bloque de balance: producción y gasto del mes con semáforo.
    fn show_energy_balance(&mut self, ui: &mut egui::Ui) {
        let cur = &self.settings.currency;
        let produccion = self.sim.energy.production_mw();
        let media = self.sim.energy.effective_capacity_mw();
        let nominal = self.sim.energy.installed_mw();
        let demanda = self.sim.energy.demand_mw;
        let escala = nominal.max(demanda).max(1.0);
        let margen = produccion - demanda;

        // Semáforo: verde con margen holgado, naranja al límite (<10 %) y rojo
        // si no llega a cubrir la demanda.
        let color = if margen < 0.0 {
            egui::Color32::from_rgb(214, 84, 72)
        } else if margen < demanda * 0.10 {
            egui::Color32::from_rgb(220, 150, 60)
        } else {
            egui::Color32::from_rgb(96, 186, 120)
        };

        ui.strong(self.i18n.t("energy.balance_title"));
        // La barra de producción mide la potencia nominal; el tramo sólido es
        // lo que produce este mes y el translúcido, la capacidad que no rinde.
        barra_energia(
            ui,
            self.i18n.t("energy.production"),
            produccion,
            nominal,
            escala,
            color,
            &self.i18n.tf(
                "energy.production_value",
                &[&cur.format_int(produccion as f64)],
            ),
            &self.i18n.tf(
                "energy.production_detail",
                &[
                    &cur.format_int(media as f64),
                    &cur.format_int(nominal as f64),
                ],
            ),
        )
        .on_hover_text(self.i18n.tf(
            "energy.production_tip_block",
            &[
                &cur.format_int(produccion as f64),
                &cur.format_int(media as f64),
                &cur.format_int(nominal as f64),
                &cur.format_int(self.sim.energy.monthly_production_mwh()),
                &cur.format_decimal(self.sim.energy.coverage() as f64, 0),
            ],
        ));
        barra_energia(
            ui,
            self.i18n.t("energy.demand"),
            demanda,
            demanda,
            escala,
            egui::Color32::from_rgb(88, 150, 214),
            &self.i18n.tf("energy.demand_value", &[&cur.format_int(demanda as f64)]),
            "",
        )
        .on_hover_text(self.i18n.t("energy.demand_tip_block"));
        ui.add_space(6.0);
    }

    /// Tabla de instalaciones actuales, con fila de totales.
    fn show_energy_installed(&mut self, ui: &mut egui::Ui) -> Option<(EnergySource, bool)> {
        let cur = &self.settings.currency;
        let mut pending = None;

        ui.strong(self.i18n.t("energy.installed_title"));
        let instaladas: Vec<EnergySource> = EnergySource::ALL
            .iter()
            .copied()
            .filter(|src| self.sim.energy.plant_count(*src) > 0)
            .collect();

        if instaladas.is_empty() {
            ui.label(self.i18n.t("energy.installed_empty"));
            return None;
        }

        egui::Grid::new("energy_installed_grid")
            .striped(true)
            .num_columns(8)
            .min_col_width(58.0)
            .spacing([16.0, 7.0])
            .show(ui, |ui| {
                ui.strong(self.i18n.t("energy.col.source"));
                ui.strong(self.i18n.t("energy.col.plants"));
                ui.strong(self.i18n.t("energy.col.production"));
                ui.strong(self.i18n.t("energy.col.cost_per_mw"));
                ui.strong(self.i18n.t("energy.col.upkeep_total"));
                ui.strong(self.i18n.t("energy.col.land_total"));
                ui.strong(self.i18n.t("energy.col.emissions"));
                ui.strong(self.i18n.t("energy.col.dismantle"));
                ui.end_row();

                for source in instaladas {
                    let spec = source.spec();
                    etiqueta_fuente(ui, self.i18n.t(source.name_key()), source);
                    ui.label(cur.format_int(self.sim.energy.plant_count(source) as f64));

                    let produccion_tip = self.i18n.tf(
                        "energy.production_tip",
                        &[
                            &cur.format_int(self.sim.energy.installed_mw_of(source) as f64),
                            &cur.format_decimal(spec.capacity_factor as f64 * 100.0, 0),
                            &cur.format_int(self.sim.energy.effective_mw_of(source) as f64),
                        ],
                    );
                    ui.label(format!(
                        "{} MW",
                        cur.format_int(self.sim.energy.production_mw_of(source) as f64)
                    ))
                    .on_hover_text(produccion_tip);

                    let coste_mw = self.sim.energy.cost_per_mw_of(source);
                    let coste_tip = self.i18n.tf(
                        "energy.cost_per_mw_tip",
                        &[
                            &cur.format_money(coste_mw),
                            &cur.format_money(self.sim.energy.monthly_upkeep_of(source)),
                            &cur.format_money(
                                self.sim.energy.monthly_emissions_t_of(source)
                                    * crate::core::energy::EMISSIONS_COST_PER_TONNE,
                            ),
                        ],
                    );
                    ui.label(cur.format_money(coste_mw))
                        .on_hover_text(coste_tip);
                    ui.label(cur.format_money(self.sim.energy.monthly_upkeep_of(source)));
                    ui.label(format!(
                        "{} ha",
                        cur.format_decimal(self.sim.energy.land_of(source) as f64, 0)
                    ));
                    let emisiones = self.sim.energy.monthly_emissions_t_of(source);
                    if emisiones > 0.0 {
                        ui.label(cur.format_int(emisiones));
                    } else {
                        ui.label(self.i18n.t("energy.clean"));
                    }

                    let refund =
                        cur.format_money((spec.capital_per_plant() * DEMOLISH_REFUND) as f64);
                    let demolish_tip = self.i18n.tf("energy.demolish_tip", &[&refund]);
                    if ui
                        .button(self.i18n.t("energy.demolish_button"))
                        .hand(ui)
                        .on_hover_text(demolish_tip)
                        .clicked()
                    {
                        pending = Some((source, false));
                    }
                    ui.end_row();
                }

                // Fila de totales: en negrita y con el color de acento, para
                // que se distinga de las fuentes de arriba.
                let acento = ui.visuals().hyperlink_color;
                let total = |ui: &mut egui::Ui, texto: String| {
                    ui.label(egui::RichText::new(texto).strong().color(acento));
                };
                let produccion = self.sim.energy.production_mw();
                let coste = self.sim.energy.monthly_upkeep() + self.sim.energy.emissions_cost();
                total(ui, self.i18n.t("energy.total_row").to_owned());
                total(ui, cur.format_int(self.sim.energy.total_plants() as f64));
                total(ui, format!("{} MW", cur.format_int(produccion as f64)));
                total(
                    ui,
                    cur.format_money(if produccion > 0.0 {
                        coste / produccion as f64
                    } else {
                        0.0
                    }),
                );
                total(ui, cur.format_money(self.sim.energy.monthly_upkeep()));
                total(
                    ui,
                    format!(
                        "{} ha",
                        cur.format_decimal(self.sim.energy.land_hectareas() as f64, 0)
                    ),
                );
                total(ui, cur.format_int(self.sim.energy.monthly_emissions_t()));
                ui.end_row();
            });

        pending
    }

    /// Catálogo de instalaciones que se pueden construir.
    fn show_energy_catalog(&mut self, ui: &mut egui::Ui) -> Option<(EnergySource, bool)> {
        let cur = &self.settings.currency;
        let mut pending = None;

        ui.strong(self.i18n.t("energy.build_title"))
            .on_hover_text(self.i18n.t("energy.build_hint"));
        ui.add_space(4.0);

        egui::Grid::new("energy_catalog_grid")
            .striped(true)
            .num_columns(9)
            .min_col_width(50.0)
            .spacing([13.0, 7.0])
            .show(ui, |ui| {
                ui.strong(self.i18n.t("energy.col.source"));
                ui.strong(self.i18n.t("energy.col.size"));
                ui.strong(self.i18n.t("energy.col.output"));
                ui.strong(self.i18n.t("energy.col.cost_per_mw"));
                ui.strong(self.i18n.t("energy.col.build"));
                ui.strong(self.i18n.t("energy.col.upkeep"));
                ui.strong(self.i18n.t("energy.col.land"));
                ui.strong(self.i18n.t("energy.col.co2"));
                ui.strong(self.i18n.t("energy.col.build_action"));
                ui.end_row();

                for source in EnergySource::ALL {
                    let spec = source.spec();
                    etiqueta_fuente(ui, self.i18n.t(source.name_key()), source);

                    ui.label(self.i18n.tf(
                        "energy.mw",
                        &[&cur.format_decimal(spec.plant_mw as f64, 0)],
                    ));

                    let output_tip = self.i18n.tf(
                        "energy.output_tip",
                        &[&cur.format_decimal(spec.capacity_factor as f64 * 100.0, 0)],
                    );
                    ui.label(self.i18n.tf(
                        "energy.mw",
                        &[&cur.format_decimal(spec.effective_mw_per_plant() as f64, 0)],
                    ))
                    .on_hover_text(output_tip);

                    let coste_mw_tip = self.i18n.tf(
                        "energy.catalog_cost_tip",
                        &[
                            &cur.format_money(spec.running_cost_per_effective_mw() as f64),
                            &cur.format_decimal(spec.effective_mw_per_1000() as f64, 1),
                        ],
                    );
                    ui.label(cur.format_money(spec.running_cost_per_effective_mw() as f64))
                        .on_hover_text(coste_mw_tip);

                    let build_tip = self.i18n.tf(
                        "energy.build_tip",
                        &[
                            &cur.format_money(spec.capital_per_effective_mw() as f64),
                            &cur.format_decimal(spec.effective_mw_per_1000() as f64, 1),
                        ],
                    );
                    ui.label(cur.format_money(spec.capital_per_plant() as f64))
                        .on_hover_text(build_tip);

                    ui.label(cur.format_money(spec.upkeep_per_plant() as f64));
                    ui.label(format!(
                        "{} ha",
                        cur.format_decimal(spec.land_per_plant() as f64, 0)
                    ));

                    let co2 = if spec.emissions_kg_per_mwh > 0.0 {
                        cur.format_decimal(spec.emissions_kg_per_mwh as f64, 0)
                    } else {
                        self.i18n.t("energy.clean").to_owned()
                    };
                    ui.label(co2);

                    if ui
                        .button(self.i18n.t("energy.build_button"))
                        .hand(ui)
                        .on_hover_text(self.i18n.t("energy.install_tip"))
                        .clicked()
                    {
                        pending = Some((source, true));
                    }
                    ui.end_row();
                }
            });

        pending
    }

    /// Panel de deuda: estado del crédito, petición de préstamo y amortización.
    /// Reutiliza el mismo aviso (`notice`) que el panel de servicios.
    fn show_debt(&mut self, ui: &mut egui::Ui) {
        let cur = &self.settings.currency;
        let limit = self.sim.economy.credit_limit(&self.sim.demographics);
        let available = self.sim.economy.available_credit(&self.sim.demographics);
        let debt = self.sim.economy.debt;
        let rate = self.sim.economy.interest_rate(&self.sim.demographics);
        let interest = self.sim.economy.monthly_interest(&self.sim.demographics);
        let bankrupt = self.sim.is_bankrupt();

        ui.heading(self.i18n.t("debt.title").to_owned());
        ui.separator();
        ui.label(self.i18n.tf("debt.balance_line", &[&cur.format_money(debt)]));
        ui.label(self.i18n.tf(
            "debt.limit_line",
            &[&cur.format_money(limit), &cur.format_money(available)],
        ));
        ui.label(self.i18n.tf(
            "debt.interest_line",
            &[
                &cur.format_decimal(rate as f64 * 100.0, 2),
                &cur.format_money(interest),
            ],
        ));

        if bankrupt {
            ui.add_space(4.0);
            ui.colored_label(egui::Color32::RED, self.i18n.t("debt.bankrupt"));
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                self.i18n.t("debt.bankrupt_effects"),
            );
        }

        ui.add_space(8.0);
        // Importe a operar: por defecto el crédito disponible (pedir) o la
        // deuda viva (amortizar), y el jugador lo ajusta con el deslizador.
        let max_op = available.max(debt).max(1.0);
        let amount_text = self.i18n.t("debt.amount").to_owned();
        ui.add(
            egui::Slider::new(&mut self.loan_request, 0.0..=max_op)
                .text(amount_text)
                .logarithmic(false),
        )
        .hand(ui);
        ui.label(self.i18n.tf(
            "debt.amount_hint",
            &[&cur.format_money(self.loan_request)],
        ));

        let mut action: Option<bool> = None; // Some(true) = pedir, Some(false) = amortizar
        ui.horizontal(|ui| {
            let pedir = self.i18n.t("debt.borrow").to_owned();
            if ui
                .add_enabled(available > 0.0, egui::Button::new(pedir))
                .hand(ui)
                .on_hover_text(self.i18n.t("debt.borrow_tip"))
                .clicked()
            {
                action = Some(true);
            }
            let amortizar = self.i18n.t("debt.repay").to_owned();
            if ui
                .add_enabled(debt > 0.0, egui::Button::new(amortizar))
                .hand(ui)
                .on_hover_text(self.i18n.t("debt.repay_tip"))
                .clicked()
            {
                action = Some(false);
            }
        });

        if let Some(pedir) = action {
            let importe = if self.loan_request > 0.0 {
                self.loan_request
            } else {
                available.max(1.0)
            };
            self.notice = Some(match pedir {
                true => self
                    .sim
                    .borrow(importe)
                    .map(|()| Notice::Borrowed { amount: importe })
                    .unwrap_or_else(Notice::Failed),
                false => self
                    .sim
                    .repay(importe)
                    .map(|amount| Notice::Repaid { amount })
                    .unwrap_or_else(Notice::Failed),
            });
            self.loan_request = 0.0;
        }

        if let Some(notice) = self.notice {
            ui.add_space(6.0);
            let (text, color) = self.describe_notice(notice);
            ui.colored_label(color, text);
        }
    }

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
                        if ui.button("+").hand(ui).on_hover_text(build_tip).clicked() {
                            pending = Some((kind, step));
                        }
                        let refund = cur.format_money(
                            (step * spec.capital_cost_per_unit * DEMOLISH_REFUND) as f64,
                        );
                        let demolish_tip =
                            self.i18n.tf("services.demolish_tip", &[&step_text, &refund, &land_text]);
                        if ui.button("-").hand(ui).on_hover_text(demolish_tip).clicked() {
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
            Notice::PowerBuilt { source, cost } => (
                self.i18n
                    .tf("energy.notice.built", &[self.i18n.t(source.name_key()), &cur.format_money(cost)]),
                egui::Color32::GREEN,
            ),
            Notice::PowerDemolished { source, refund } => (
                self.i18n.tf(
                    "energy.notice.demolished",
                    &[self.i18n.t(source.name_key()), &cur.format_money(refund)],
                ),
                egui::Color32::LIGHT_BLUE,
            ),
            Notice::Borrowed { amount } => (
                self.i18n
                    .tf("debt.notice.borrowed", &[&cur.format_money(amount)]),
                egui::Color32::LIGHT_BLUE,
            ),
            Notice::Repaid { amount } => (
                self.i18n
                    .tf("debt.notice.repaid", &[&cur.format_money(amount)]),
                egui::Color32::GREEN,
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
            .id(egui::Id::new("modal_restart"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(confirm);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(accept).hand(ui).clicked() {
                        restart = true;
                    }
                    if ui.button(&cancel).hand(ui).clicked() {
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
            .id(egui::Id::new("modal_info"))
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
        let theme_label = self.i18n.t("settings.theme").to_owned();
        let lang_before = self.settings.lang;

        let mut open = true;
        egui::Window::new(title)
            .id(egui::Id::new("modal_settings"))
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
                            ui.selectable_value(&mut self.settings.lang, lang, name).hand(ui);
                        }
                    })
                    .response
                    .hand(ui);

                ui.add_space(8.0);
                ui.label(theme_label);
                egui::ComboBox::from_id_salt("settings_theme")
                    .selected_text(self.i18n.t(self.settings.theme.name_key()))
                    .show_ui(ui, |ui| {
                        for theme in Theme::ALL {
                            let name = self.i18n.t(theme.name_key());
                            ui.selectable_value(&mut self.settings.theme, theme, name).hand(ui);
                        }
                    })
                    .response
                    .hand(ui);

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
                            )
                            .hand(ui);
                        }
                    })
                    .response
                    .hand(ui);

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
fn error_key(err: ActionError) -> &'static str {
    match err {
        ActionError::InsufficientFunds => "action.error.no_money",
        ActionError::InsufficientLand => "action.error.no_land",
        ActionError::NoCapacity => "action.error.no_units",
        ActionError::CreditLimitExceeded => "action.error.credit_limit",
        ActionError::NoDebt => "action.error.no_debt",
        ActionError::InvalidAmount => "action.error.invalid_amount",
    }
}

fn plot_money(ui: &mut egui::Ui, data: &[f64], no_data: &str) {
    plot_generic(ui, data, no_data);
}

/// Fila del balance eléctrico: etiqueta, barra de dos tramos y cifras.
///
/// `solido` es lo que se pinta a color lleno; `total` la longitud de la barra
/// (el tramo entre ambos se dibuja translúcido). `detalle` va en texto tenue
/// a la derecha y puede ir vacío.
#[allow(clippy::too_many_arguments)]
fn barra_energia(
    ui: &mut egui::Ui,
    etiqueta: &str,
    solido: f32,
    total: f32,
    escala: f32,
    color: egui::Color32,
    valor: &str,
    detalle: &str,
) -> egui::Response {
    ui.horizontal(|ui| {
        ui.add_sized([78.0, 14.0], egui::Label::new(etiqueta));
        let (rect, _) = ui.allocate_exact_size(egui::vec2(240.0, 12.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 3.0, ui.visuals().extreme_bg_color);
        let ancho = |mw: f32| (mw / escala).clamp(0.0, 1.0) * rect.width();
        if total > solido {
            let tramo = egui::Rect::from_min_size(rect.min, egui::vec2(ancho(total), rect.height()));
            ui.painter()
                .rect_filled(tramo, 3.0, color.gamma_multiply(0.35));
        }
        let relleno = egui::Rect::from_min_size(rect.min, egui::vec2(ancho(solido), rect.height()));
        ui.painter().rect_filled(relleno, 3.0, color);
        ui.label(egui::RichText::new(valor).color(color));
        if !detalle.is_empty() {
            ui.label(egui::RichText::new(detalle).weak());
        }
    })
    .response
}

/// Separa «icono nombre» en sus dos partes (sin espacio, todo es nombre).
fn partir_etiqueta(texto: &str) -> (&str, &str) {
    match texto.split_once(' ') {
        Some((icono, nombre)) => (icono, nombre),
        None => ("", texto),
    }
}

/// Pinta la etiqueta de una fuente: el icono con su color propio y el nombre
/// con el color de texto normal.
fn etiqueta_fuente(ui: &mut egui::Ui, texto: &str, source: EnergySource) {
    let (icono, nombre) = partir_etiqueta(texto);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if !icono.is_empty() {
            ui.label(egui::RichText::new(icono).color(theme::color_fuente(source)));
        }
        ui.label(nombre);
    });
}

fn plot_generic(ui: &mut egui::Ui, data: &[f64], no_data: &str) {
    use egui::{Pos2, Sense, Stroke};
    if data.is_empty() {
        ui.label(no_data);
        return;
    }
    // Los colores salen del tema activo: así las gráficas siguen siendo
    // legibles con fondo claro, sepia o azul noche.
    let fondo = ui.visuals().extreme_bg_color;
    let linea = ui.visuals().hyperlink_color;
    let eje = ui.visuals().weak_text_color();
    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (max - min).max(1e-6);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 180.0), Sense::hover());
    ui.painter().rect_filled(rect, 4.0, fondo);
    let n = data.len();
    let mut prev: Option<Pos2> = None;
    for (i, v) in data.iter().enumerate() {
        let x = rect.min.x + rect.width() * (i as f32 / (n.max(2) - 1) as f32);
        let t = ((v - min) / range) as f32;
        let y = rect.max.y - 8.0 - t * (rect.height() - 16.0);
        let p = Pos2::new(x, y);
        if let Some(q) = prev {
            ui.painter().line_segment([q, p], Stroke::new(2.0_f32, linea));
        }
        prev = Some(p);
    }
    // Eje min/max
    ui.painter().text(
        Pos2::new(rect.min.x + 4.0, rect.min.y + 4.0),
        egui::Align2::LEFT_TOP,
        format!("max {:.0}", max),
        egui::FontId::proportional(12.0),
        eje,
    );
    ui.painter().text(
        Pos2::new(rect.min.x + 4.0, rect.max.y - 4.0),
        egui::Align2::LEFT_BOTTOM,
        format!("min {:.0}", min),
        egui::FontId::proportional(12.0),
        eje,
    );
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Renderiza la UI sin ventana: detecta pánicos de layout/borrow en todas
    /// las pestañas (incluidas la tabla de servicios y el panel de deuda) sin
    /// abrir una ventana, en ambos idiomas y también en quiebra.
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
        type Escenario = (&'static str, fn(&mut MacroSimApp));
        let escenarios: [Escenario; 6] = [
            ("normal", |_| {}),
            ("energía: construir", |app| {
                app.tab = Tab::Energia;
                app.energy_tab = EnergyTab::Construir;
            }),
            ("energía: demanda", |app| {
                app.tab = Tab::Energia;
                app.energy_tab = EnergyTab::Demanda;
            }),
            ("sin centrales", |app| {
                for source in EnergySource::ALL {
                    app.sim.energy.get_mut(source).plants = 0;
                }
                app.sim.refresh_costs();
            }),
            ("quiebra", |app| {
                let limite = app.sim.economy.credit_limit(&app.sim.demographics);
                app.sim.economy.debt = limite * 1.5;
                app.sim.economy.money = 0.0;
            }),
            ("inglés", |app| {
                app.settings.apply_language(Lang::En);
                app.i18n.set_lang(Lang::En);
            }),
        ];
        for (nombre, preparar) in escenarios {
            preparar(&mut app);
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
                assert!(
                    !output.shapes.is_empty(),
                    "la pestaña {tab:?} ({nombre}) no dibujó nada"
                );
            }
        }
    }

    /// La ventana de Configuración no debe moverse al cambiar de idioma: su
    /// posición se guarda en memoria por Id, y el Id no puede depender del
    /// título traducido.
    #[test]
    fn la_ventana_de_configuracion_no_se_mueve_al_cambiar_idioma() {
        let ctx = egui::Context::default();
        let mut app = MacroSimApp {
            modal: Some(Modal::Settings),
            ..MacroSimApp::default()
        };
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 720.0),
            )),
            ..Default::default()
        };
        let id = egui::Id::new("modal_settings");

        let _ = ctx.run(input.clone(), |ctx| app.ui(ctx));
        let es = ctx.memory(|m| m.area_rect(id)).expect("la ventana debe existir");

        app.settings.apply_language(Lang::En);
        app.i18n.set_lang(Lang::En);
        let _ = ctx.run(input, |ctx| app.ui(ctx));
        let en = ctx.memory(|m| m.area_rect(id)).expect("la ventana debe seguir existiendo");

        assert_eq!(es, en, "la ventana cambió de sitio al cambiar de idioma");
    }

    /// Al pasar el ratón por un botón, egui debe pedir el cursor de mano.
    /// Se comprueba con el cursor sin `hand()` como control negativo.
    #[test]
    fn el_cursor_cambia_a_mano_sobre_un_boton() {
        fn cursor_en(ctx: &egui::Context, con_mano: bool) -> egui::CursorIcon {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(320.0, 240.0),
                )),
                events: vec![egui::Event::PointerMoved(egui::pos2(30.0, 30.0))],
                ..Default::default()
            };
            let mut cursor = egui::CursorIcon::Default;
            // Dos pasadas: la primera da de alta el widget, la segunda ya sabe
            // si el puntero está encima.
            for _ in 0..2 {
                let output = ctx.run(input.clone(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let response = ui.button("boton");
                        if con_mano {
                            response.hand(ui);
                        }
                    });
                });
                cursor = output.platform_output.cursor_icon;
            }
            cursor
        }

        let ctx = egui::Context::default();
        assert_eq!(
            cursor_en(&ctx, false),
            egui::CursorIcon::Default,
            "sin `hand()` egui no pide cursor de mano (lo que pasaba antes)"
        );

        let ctx = egui::Context::default();
        assert_eq!(
            cursor_en(&ctx, true),
            egui::CursorIcon::PointingHand,
            "con `hand()` el botón debe mostrar la mano"
        );
    }

    /// Las cuatro paletas deben ser distintas entre sí y la azul noche debe ser
    /// la de por defecto y la primera del desplegable.
    #[test]
    fn cada_paleta_aplica_colores_distintos_y_midnight_va_primera() {
        assert_eq!(Theme::default(), Theme::Midnight);
        assert_eq!(Theme::ALL[0], Theme::Midnight);
        assert_eq!(Settings::default().theme, Theme::Midnight);

        let ctx = egui::Context::default();
        let mut fondos = Vec::new();
        for theme in Theme::ALL {
            theme::apply(&ctx, theme);
            fondos.push(ctx.style().visuals.panel_fill);
        }
        for (i, a) in fondos.iter().enumerate() {
            for b in fondos.iter().skip(i + 1) {
                assert_ne!(a, b, "dos paletas comparten color de fondo");
            }
        }
    }

    /// Cada fuente de energía debe poder teñirse con un color propio y distinto:
    /// es la única forma de tener "iconos de color" (epaint no soporta fuentes
    /// de color).
    #[test]
    fn cada_fuente_tiene_color_propio() {
        let colores: Vec<egui::Color32> = EnergySource::ALL
            .iter()
            .map(|s| theme::color_fuente(*s))
            .collect();
        for (i, a) in colores.iter().enumerate() {
            for b in colores.iter().skip(i + 1) {
                assert_ne!(a, b, "dos fuentes comparten color de icono");
            }
        }
    }

    /// La etiqueta de cada fuente debe separarse en icono + nombre.
    #[test]
    fn la_etiqueta_de_fuente_separa_icono_y_nombre() {
        let i18n = I18n::new(Lang::Es);
        for source in EnergySource::ALL {
            let (icono, nombre) = partir_etiqueta(i18n.t(source.name_key()));
            assert!(!icono.is_empty(), "{} sin icono", source.name_key());
            assert!(!nombre.is_empty(), "{} sin nombre", source.name_key());
        }
        // Sin icono, todo el texto es el nombre.
        assert_eq!(partir_etiqueta("Carbón"), ("", "Carbón"));
    }

    /// El semáforo del balance: verde con margen holgado, naranja al límite
    /// (<10 %) y rojo si no se cubre la demanda.
    #[test]
    fn el_balance_cambia_de_color_segun_el_margen() {
        fn colores_de(shape: &egui::Shape, salida: &mut Vec<egui::Color32>) {
            match shape {
                egui::Shape::Rect(r) => salida.push(r.fill),
                egui::Shape::Vec(v) => {
                    for s in v {
                        colores_de(s, salida);
                    }
                }
                _ => {}
            }
        }

        fn pintar(demanda: f32) -> Vec<egui::Color32> {
            let ctx = egui::Context::default();
            let mut app = MacroSimApp {
                tab: Tab::Energia,
                ..MacroSimApp::default()
            };
            app.sim.energy.demand_mw = demanda;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 720.0),
                )),
                ..Default::default()
            };
            let output = ctx.run(input, |ctx| app.ui(ctx));
            let mut colores = Vec::new();
            for clipped in &output.shapes {
                colores_de(&clipped.shape, &mut colores);
            }
            colores
        }

        let verde = egui::Color32::from_rgb(96, 186, 120);
        let naranja = egui::Color32::from_rgb(220, 150, 60);
        let rojo = egui::Color32::from_rgb(214, 84, 72);

        // 480 MW de producción: margen holgado con 420, justo con 470, déficit con 600.
        assert!(pintar(420.0).contains(&verde), "con margen amplio debe ser verde");
        assert!(pintar(470.0).contains(&naranja), "al límite debe ser naranja");
        assert!(pintar(600.0).contains(&rojo), "sin cubrir debe ser rojo");
        assert!(!pintar(600.0).contains(&verde), "en déficit no debe haber verde de producción");
    }

    /// Ningún texto de la UI puede usar un carácter que el build web no sepa
    /// dibujar: en web solo existen las fuentes de egui más nuestros dos
    /// subsets, así que un emoji fuera de ese juego se convierte en un
    /// cuadradito. Este test reproduce ese arranque exacto y revisa todos los
    /// textos de los dos idiomas.
    #[test]
    fn ningun_icono_se_queda_sin_glifo_en_el_build_web() {
        let ctx = egui::Context::default();
        crate::fuentes_embebidas(&ctx);
        let font = egui::FontId::proportional(14.0);

        let mut faltan = Vec::new();
        // `Context::fonts` solo está disponible tras un frame: se hace uno vacío.
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(640.0, 480.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            for json in [crate::i18n::LOCALES_ES, crate::i18n::LOCALES_EN] {
                let textos: std::collections::HashMap<String, String> =
                    serde_json::from_str(json).expect("locales válidos");
                for (clave, texto) in textos {
                    for ch in texto.chars() {
                        if ch.is_ascii() || ch == '\u{fe0f}' {
                            continue;
                        }
                        if !ctx.fonts(|f| f.has_glyph(&font, ch)) {
                            faltan.push(format!("{ch} (U+{:04X}) en '{clave}'", ch as u32));
                        }
                    }
                }
            }
        });
        faltan.sort();
        faltan.dedup();
        assert!(
            faltan.is_empty(),
            "glifos que el build web no puede dibujar: {faltan:#?}"
        );
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
                ActionError::InsufficientFunds,
                ActionError::InsufficientLand,
                ActionError::NoCapacity,
                ActionError::CreditLimitExceeded,
                ActionError::NoDebt,
                ActionError::InvalidAmount,
            ] {
                let key = error_key(err);
                assert_ne!(i18n.t(key), key, "falta el error {err:?} en {lang:?}");
            }
        }
    }
}

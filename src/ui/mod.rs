use crate::core::settings::{Currency, Settings};
use crate::core::Simulation;
use crate::i18n::{I18n, Lang};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Resumen,
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

pub struct MacroSimApp {
    pub sim: Simulation,
    pub settings: Settings,
    pub i18n: I18n,
    pub tab: Tab,
    pub modal: Option<Modal>,
}

impl Default for MacroSimApp {
    fn default() -> Self {
        Self {
            sim: Simulation::default(),
            settings: Settings::default(),
            i18n: I18n::new(Lang::default()),
            tab: Tab::Resumen,
            modal: None,
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
                let income_text = self.i18n.t("panel.income_tax").to_owned();
                ui.add(
                    egui::Slider::new(&mut self.sim.economy.income_tax, 0.0..=100.0)
                        .text(income_text),
                );
                let spend_text = self.i18n.tf("panel.spending", &[&self.settings.currency.symbol]);
                ui.add(
                    egui::Slider::new(&mut self.sim.economy.public_spending, 0.0..=20.0)
                        .text(spend_text),
                );

                let balance = self.sim.economy.monthly_balance();
                ui.separator();
                ui.label(self.i18n.tf(
                    "panel.balance",
                    &[&self.settings.currency.format_money(balance)],
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
                let economia = self.i18n.t("tab.economy").to_owned();
                let energia = self.i18n.t("tab.energy").to_owned();
                let informes = self.i18n.t("tab.reports").to_owned();
                ui.selectable_value(&mut self.tab, Tab::Resumen, resumen);
                ui.selectable_value(&mut self.tab, Tab::Economia, economia);
                ui.selectable_value(&mut self.tab, Tab::Energia, energia);
                ui.selectable_value(&mut self.tab, Tab::Informes, informes);
            });
            ui.separator();

            let no_data = self.i18n.t("plot.no_data").to_owned();
            match self.tab {
                Tab::Resumen => {
                    ui.heading(self.i18n.t("summary.budget_evolution"));
                    plot_money(ui, &self.sim.history.money, &no_data);
                    ui.add_space(8.0);
                    ui.heading(self.i18n.t("summary.unemployment"));
                    plot_generic(ui, &self.sim.history.unemployment, &no_data);
                }
                Tab::Economia => {
                    ui.heading(self.i18n.t("economy.report"));
                    ui.label(self.i18n.tf(
                        "economy.incomes_note",
                        &[
                            &self.settings.currency.format_decimal(
                                self.sim.economy.corporate_tax as f64,
                                1,
                            ),
                            &self
                                .settings
                                .currency
                                .format_decimal(self.sim.economy.income_tax as f64, 1),
                        ],
                    ));
                    plot_money(ui, &self.sim.history.money, &no_data);
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
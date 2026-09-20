use crate::core::Simulation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Resumen,
    Economia,
    Energia,
    Informes,
}

pub struct MacroSimApp {
    pub sim: Simulation,
    pub tab: Tab,
}

impl Default for MacroSimApp {
    fn default() -> Self {
        Self {
            sim: Simulation::default(),
            tab: Tab::Resumen,
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
                ui.heading("🏛️ MacroSim");
                ui.separator();
                ui.label(format!(
                    "📅 {:02}/{:04}  |  💰 ${:.0}  |  👥 {}  |  📉 Paro {:.1}%",
                    self.sim.economy.month,
                    self.sim.economy.year,
                    self.sim.economy.money,
                    self.sim.economy.population,
                    self.sim.economy.unemployment_rate
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = if self.sim.paused { "▶ Reanudar" } else { "⏸ Pausa" };
                    if ui.button(label).clicked() {
                        self.sim.paused = !self.sim.paused;
                    }
                    ui.add(
                        egui::Slider::new(&mut self.sim.speed, 1..=12)
                            .text("meses/seg"),
                    );
                    if ui.button("＋ 1 mes").clicked() {
                        self.sim.tick_month();
                    }
                });
            });
        });

        // Panel lateral: políticas fiscales y gasto (subir/bajar impuestos y gastos)
        egui::SidePanel::left("dashboard_panel")
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("📊 Estado del Estado");
                ui.separator();

                ui.label(format!("Presupuesto: ${:.2}", self.sim.economy.money));
                ui.label(format!("Población: {}", self.sim.economy.population));
                ui.label(format!(
                    "Desempleo: {:.1}%",
                    self.sim.economy.unemployment_rate
                ));

                ui.add_space(10.0);
                ui.heading("⚡ Red Eléctrica");
                ui.label(format!("Capacidad: {:.0} MW", self.sim.energy.capacity_mw));
                ui.label(format!("Demanda: {:.0} MW", self.sim.energy.demand_mw));
                ui.label(format!("Precio: {:.1} €/MWh", self.sim.energy.price_per_mwh));
                ui.add(
                    egui::Slider::new(&mut self.sim.energy.capacity_mw, 0.0..=2000.0)
                        .text("Capacidad (MW)"),
                );

                ui.add_space(15.0);
                ui.heading("⚙️ Políticas");
                ui.add(
                    egui::Slider::new(&mut self.sim.economy.corporate_tax, 0.0..=100.0)
                        .text("Impuesto Corp. (%)"),
                );
                ui.add(
                    egui::Slider::new(&mut self.sim.economy.income_tax, 0.0..=100.0)
                        .text("IRPF (%)"),
                );
                ui.add(
                    egui::Slider::new(&mut self.sim.economy.public_spending, 0.0..=20.0)
                        .text("Gasto/hab. ($/mes)"),
                );

                let balance = self.sim.economy.population as f64
                    * (self.sim.economy.corporate_tax as f64 * 0.5
                        + self.sim.economy.income_tax as f64 * 0.8
                        - self.sim.economy.public_spending as f64);
                ui.separator();
                ui.label(format!("Balance mensual est.: ${:.0}", balance));
                if balance < 0.0 {
                    ui.colored_label(egui::Color32::RED, "⚠️ Déficit");
                } else {
                    ui.colored_label(egui::Color32::GREEN, "✓ Superávit");
                }
            });

        // Panel central: informes por pestañas
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Resumen, "📈 Resumen");
                ui.selectable_value(&mut self.tab, Tab::Economia, "💶 Economía");
                ui.selectable_value(&mut self.tab, Tab::Energia, "⚡ Energía");
                ui.selectable_value(&mut self.tab, Tab::Informes, "📋 Informes");
            });
            ui.separator();

            match self.tab {
                Tab::Resumen => {
                    ui.heading("Evolución del presupuesto");
                    plot_money(ui, &self.sim.history.money);
                    ui.add_space(8.0);
                    ui.heading("Desempleo (%)");
                    plot_generic(ui, &self.sim.history.unemployment);
                }
                Tab::Economia => {
                    ui.heading("💶 Informe económico");
                    ui.label(format!(
                        "Ingresos dependen de impuesto de sociedades ({:.1}%) e IRPF ({:.1}%).",
                        self.sim.economy.corporate_tax, self.sim.economy.income_tax
                    ));
                    plot_money(ui, &self.sim.history.money);
                    plot_generic(ui, &self.sim.history.unemployment);
                }
                Tab::Energia => {
                    ui.heading("⚡ Informe energético");
                    let coverage = if self.sim.energy.capacity_mw > 0.0 {
                        self.sim.energy.demand_mw / self.sim.energy.capacity_mw * 100.0
                    } else {
                        0.0
                    };
                    ui.label(format!("Cobertura demanda: {:.1}%", coverage));
                    if self.sim.energy.demand_mw > self.sim.energy.capacity_mw {
                        ui.colored_label(
                            egui::Color32::RED,
                            "⚠️ Déficit energético: el paro subirá.",
                        );
                    }
                    plot_generic(ui, &self.sim.history.demand);
                }
                Tab::Informes => {
                    ui.heading("📋 Últimos 12 meses");
                    egui::Grid::new("informe_grid")
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Mes");
                            ui.label("Presupuesto");
                            ui.label("Paro %");
                            ui.end_row();
                            let n = self.sim.history.money.len().min(12);
                            for i in 0..n {
                                let idx = self.sim.history.money.len() - n + i;
                                ui.label(format!("-{}", n - i));
                                ui.label(format!(
                                    "${:.0}",
                                    self.sim.history.money.get(idx).copied().unwrap_or(0.0)
                                ));
                                ui.label(format!(
                                    "{:.1}%",
                                    self.sim.history.unemployment.get(idx).copied().unwrap_or(0.0)
                                ));
                                ui.end_row();
                            }
                        });
                }
            }
        });
    }
}

fn plot_money(ui: &mut egui::Ui, data: &[f64]) {
    plot_generic(ui, data);
}

fn plot_generic(ui: &mut egui::Ui, data: &[f64]) {
    use egui::{Color32, Pos2, Sense, Stroke};
    if data.is_empty() {
        ui.label("Sin datos todavía...");
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
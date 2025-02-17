use parking_lot::Mutex;
use std::sync::Arc;

pub struct Widget {
    desc: Descriptor,
    scenario: Arc<Mutex<Option<Scenario>>>,
}

impl Widget {
    pub fn new() -> Self {
        Self {
            desc: Descriptor::new(),
            scenario: Arc::new(Mutex::new(None)),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> egui::InnerResponse<()> {
        egui::SidePanel::left("settings")
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Parameters");
                self.desc.show_inside(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Simulation");

                let text = if self.scenario.lock().is_some() {
                    egui::RichText::new("Active").color(egui::Color32::GREEN)
                } else {
                    egui::RichText::new("Inactive").color(egui::Color32::RED)
                };
                ui.label(text);

                if ui.button("New Scenario").clicked() {
                    let scenario = Scenario::new(self.desc.clone());
                    self.scenario.lock().replace(scenario);
                }

                if ui.button("Drop Scenario").clicked() {
                    self.scenario.lock().take();
                }
            });

            egui_plot::Plot::new("Plotting")
                .view_aspect(1.0)
                .data_aspect(1.0)
                .show(ui, |ui| {
                    if let Some(scenario) = self.scenario.lock().as_mut() {
                        scenario.show_inside(ui);
                    }
                });
        })
    }

    pub fn spawn_thread(&mut self) -> std::thread::JoinHandle<()> {
        const LOOP_WAIT: f64 = 0.016;

        let scenario = self.scenario.clone();
        let mut instant = None;
        std::thread::spawn(move || loop {
            'scope: {
                let scenario_guard = &mut scenario.lock();
                let Some(scenario) = scenario_guard.as_mut() else {
                    instant = None;
                    break 'scope;
                };

                let new_instant = std::time::Instant::now();
                let Some(instant) = std::mem::replace(&mut instant, Some(new_instant)) else {
                    break 'scope;
                };

                let delta_time = instant.elapsed().as_secs_f64();
                scenario.forward(delta_time);
            }

            std::thread::sleep(std::time::Duration::from_secs_f64(LOOP_WAIT));
        })
    }
}

#[derive(Clone)]
pub struct Descriptor {
    value: f64,
}

impl Descriptor {
    pub fn new() -> Self {
        Self {
            value: Default::default(),
        }
    }

    pub fn show_inside(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::Slider::new(&mut self.value, 0.0..=1.0).text("Value"));
    }
}

pub struct Scenario {
    desc: Descriptor,
    value: f64,
}

impl Scenario {
    pub fn new(desc: Descriptor) -> Self {
        Self {
            desc,
            value: Default::default(),
        }
    }

    pub fn forward(&mut self, delta_time: f64) {
        self.value += self.desc.value * delta_time;
    }

    pub fn show_inside(&mut self, ui: &mut egui_plot::PlotUi) {
        let points = egui_plot::Points::new(vec![[self.value, 0.0]])
            .radius(4.0)
            .color(egui::Color32::RED);
        ui.add(points);
    }
}

use parking_lot::Mutex;
use std::sync::Arc;

use crate::simulation::{Descriptor, Scenario};

#[derive(Clone, Debug)]
pub struct Body {
    desc: Descriptor,
    scenario: Arc<Mutex<Option<Scenario>>>,
}

impl Body {
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
        const WAIT_TIME: f64 = 0.001;

        let scenario = self.scenario.clone();
        std::thread::spawn(move || loop {
            let instant = std::time::Instant::now();

            if let Some(scenario) = scenario.lock().as_mut() {
                scenario.forward();

                let elapsed = instant.elapsed().as_secs_f64();
                let loop_wait =
                    (scenario.desc.delta_t - elapsed).max(0.0) / scenario.desc.time_scale;
                std::thread::sleep(std::time::Duration::from_secs_f64(loop_wait));
            } else {
                std::thread::sleep(std::time::Duration::from_secs_f64(WAIT_TIME));
            }
        })
    }
}

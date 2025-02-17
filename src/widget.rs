pub struct Widget {}

impl Widget {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, ctx: &egui::Context) -> egui::InnerResponse<()> {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello, world!");

            let points = egui_plot::Points::new(vec![[0.0, 0.0], [1.0, 1.0], [2.0, 0.5]])
                .radius(5.0)
                .color(egui::Color32::RED);

            egui_plot::Plot::new("Simulation")
                .view_aspect(1.0)
                .data_aspect(1.0)
                .show(ui, |ui| {
                    ui.points(points);
                });
        })
    }
}

#[derive(Clone)]
pub struct Descriptor {
    pub time_scale: f64,
    pub delta_t: f64,
    pub delta_x: f64,
    pub bound: f64,
}

impl Descriptor {
    pub fn new() -> Self {
        Self {
            time_scale: 1.0,
            delta_t: 0.01666,
            delta_x: 0.05,
            bound: 10.0,
        }
    }

    pub fn show_inside(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::Slider::new(&mut self.time_scale, 0.0..=10.0).text("Time Scale"));
        ui.add(egui::Slider::new(&mut self.delta_t, 0.0..=0.1).text("Delta Time"));
        ui.add(egui::Slider::new(&mut self.delta_x, 0.0..=0.1).text("Delta Space"));
        ui.add(egui::Slider::new(&mut self.bound, 0.0..=100.0).text("Bound"));
    }
}

pub struct Scenario {
    pub desc: Descriptor,
    u_grid: Vec<f64>,
}

impl Scenario {
    pub fn new(desc: Descriptor) -> Self {
        let n = discrete(desc.bound, &desc);
        let mut u_grid = vec![0.0; n];

        let lower = discrete(0.5, &desc).min(n);
        let upper = discrete(1.0, &desc).min(n);
        for i in lower..upper {
            u_grid[i] = 10.0;
        }

        Self { desc, u_grid }
    }

    pub fn forward(&mut self, delta_t: f64) {
        let u = self.u_grid.clone();
        let u_suc = &mut self.u_grid;

        let c = 0.5;
        let v = c * delta_t / self.desc.delta_x;
        for i in 0..u.len() {
            if i == 0 || i == u.len() - 1 {
                // Boundary condition
                u_suc[i] = u[i];
            } else {
                // Forward Euler + Central
                // u_suc[i] = u[i] - 0.5 * v * (u[i + 1] - u[i - 1]);
                //
                // Forward Euler + Lax-Friedrich
                // u_suc[i] = 0.5 * (u[i + 1] + u[i - 1]) - 0.5 * v * (u[i + 1] - u[i - 1]);
                //
                // Forward Euler + Upwind
                // u_suc[i] = u[i] - v * (u[i] - u[i - 1]);
                //
                // Forward Euler + Lax-Wendroff
                u_suc[i] = u[i] - 0.5 * v * (u[i + 1] - u[i - 1])
                    + 0.5 * v * v * (u[i + 1] - 2.0 * u[i] + u[i - 1]);
            }
        }
    }

    pub fn show_inside(&mut self, ui: &mut egui_plot::PlotUi) {
        let points = self
            .u_grid
            .iter()
            .enumerate()
            .map(|(i, x)| [i as f64, *x])
            .collect::<Vec<_>>();

        let points = egui_plot::Points::new(points)
            .radius(4.0)
            .color(egui::Color32::RED);

        ui.add(points);
    }
}

fn discrete(x: f64, desc: &Descriptor) -> usize {
    (x / desc.delta_x).round() as usize
}

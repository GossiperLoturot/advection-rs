#[derive(Clone, Copy, PartialEq, Eq, strum::EnumIter, Debug)]
pub enum SpatialScheme {
    Central,
    Upwind,
    LaxWendroff,
    ENO,
    WENO,
    CIP,
}

#[derive(Clone, Copy, PartialEq, Eq, strum::EnumIter, Debug)]
pub enum TemporalScheme {
    ForwardEuler,
    Rk2,
    Rk3,
    Rk4,
    TvdRk2,
    TvdRk3,
    TvdRk4,
}

#[derive(Clone, Debug)]
pub struct Descriptor {
    pub time_scale: f64,
    pub delta_t: f64,
    pub delta_x: f64,
    pub bound: f64,
    pub x_1: f64,
    pub x_2: f64,
    pub vel: f64,
    pub spatial_scheme: SpatialScheme,
    pub temporal_scheme: TemporalScheme,
}

impl Descriptor {
    pub fn new() -> Self {
        Self {
            time_scale: 1.0,
            delta_t: 0.01666,
            delta_x: 0.05,
            bound: 10.0,
            x_1: 2.0,
            x_2: 4.0,
            vel: 1.0,
            spatial_scheme: SpatialScheme::WENO,
            temporal_scheme: TemporalScheme::ForwardEuler,
        }
    }

    pub fn show_inside(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::Slider::new(&mut self.time_scale, 0.0..=10.0).text("Time Scale"));
        ui.add(egui::Slider::new(&mut self.delta_t, 0.0..=0.1).text("Delta Time"));
        ui.add(egui::Slider::new(&mut self.delta_x, 0.0..=0.1).text("Delta Space"));
        ui.add(egui::Slider::new(&mut self.bound, 0.0..=100.0).text("Bound"));
        ui.add(egui::Slider::new(&mut self.x_1, 0.0..=10.0).text("x1"));
        ui.add(egui::Slider::new(&mut self.x_2, 0.0..=10.0).text("x2"));
        ui.add(egui::Slider::new(&mut self.vel, 0.0..=10.0).text("Velocity"));

        let display = format!("{:?}", self.spatial_scheme);
        egui::ComboBox::from_label("Spatial Scheme")
            .selected_text(display)
            .show_ui(ui, |ui| {
                <SpatialScheme as strum::IntoEnumIterator>::iter().for_each(|scheme| {
                    let display = format!("{:?}", scheme);
                    ui.selectable_value(&mut self.spatial_scheme, scheme, display);
                });
            });

        let display = format!("{:?}", self.temporal_scheme);
        egui::ComboBox::from_label("Temporal Scheme")
            .selected_text(display)
            .show_ui(ui, |ui| {
                <TemporalScheme as strum::IntoEnumIterator>::iter().for_each(|scheme| {
                    let display = format!("{:?}", scheme);
                    ui.selectable_value(&mut self.temporal_scheme, scheme, display);
                });
            });
    }
}

#[derive(Clone, Debug)]
pub enum Buffer {
    Base {
        u: nalgebra::DVector<f64>,
    },
    CIP {
        u: nalgebra::DVector<f64>,
        g: nalgebra::DVector<f64>,
    },
}

#[derive(Clone, Debug)]
pub struct Scenario {
    pub desc: Descriptor,
    buffer: Buffer,
}

impl Scenario {
    pub fn new(desc: Descriptor) -> Self {
        let buffer = match desc.spatial_scheme {
            SpatialScheme::CIP => {
                let n = discretize(desc.bound, &desc);
                let mut u = nalgebra::DVector::zeros(n);

                u = init_square_wave(&u, &desc);

                let g = nalgebra::DVector::zeros(n);
                Buffer::CIP { u, g }
            }
            _ => {
                let n = discretize(desc.bound, &desc);
                let mut u = nalgebra::DVector::zeros(n);

                u = init_square_wave(&u, &desc);

                Buffer::Base { u }
            }
        };

        Self { desc, buffer }
    }

    pub fn forward(&mut self) {
        match (self.desc.spatial_scheme, &mut self.buffer) {
            (
                SpatialScheme::Central
                | SpatialScheme::Upwind
                | SpatialScheme::LaxWendroff
                | SpatialScheme::ENO
                | SpatialScheme::WENO,
                Buffer::Base { u },
            ) => {
                let diff_fn = match self.desc.spatial_scheme {
                    SpatialScheme::Central => central_diff,
                    SpatialScheme::Upwind => upwind_diff,
                    SpatialScheme::LaxWendroff => lax_wendroff_diff,
                    SpatialScheme::ENO => eno_diff,
                    SpatialScheme::WENO => weno_diff,
                    _ => unreachable!(),
                };

                let forward_fn = match self.desc.temporal_scheme {
                    TemporalScheme::ForwardEuler => forward_euler,
                    TemporalScheme::Rk2 => rk2,
                    TemporalScheme::Rk3 => rk3,
                    TemporalScheme::Rk4 => rk4,
                    TemporalScheme::TvdRk2 => tvd_rk2,
                    TemporalScheme::TvdRk3 => tvd_rk3,
                    TemporalScheme::TvdRk4 => tvd_rk4,
                };

                *u = forward_fn(u, diff_fn, &self.desc);
            }
            (SpatialScheme::CIP, Buffer::CIP { u, g }) => {
                (*u, *g) = cip(u, g, &self.desc);
            }
            _ => unreachable!(),
        }
    }

    pub fn show_inside(&mut self, ui: &mut egui_plot::PlotUi) {
        let u = match &self.buffer {
            Buffer::Base { u } => u,
            Buffer::CIP { u, .. } => u,
        };

        let points = u
            .iter()
            .enumerate()
            .map(|(i, y)| [i as f64 * self.desc.delta_x, *y])
            .collect::<Vec<_>>();

        let points = egui_plot::Points::new(points)
            .radius(4.0)
            .color(egui::Color32::RED);

        ui.add(points);
    }
}

fn discretize(x: f64, desc: &Descriptor) -> usize {
    (x / desc.delta_x).round() as usize
}

fn init_square_wave(u: &nalgebra::DVector<f64>, desc: &Descriptor) -> nalgebra::DVector<f64> {
    let n = u.len();

    let mut ret = nalgebra::DVector::zeros(n);

    let lower = discretize(desc.x_1, desc);
    let upper = discretize(desc.x_2, desc);
    for i in lower..upper {
        ret[i] = 1.0;
    }

    ret
}

fn forward_diff(u: &nalgebra::DVector<f64>, desc: &Descriptor) -> nalgebra::DVector<f64> {
    let n = u.len();
    let dx = desc.delta_x;
    let p = -desc.vel * desc.delta_t;

    let mut ret = nalgebra::DVector::zeros(n);

    for i in 0..n - 1 {
        ret[i] = (u[i + 1] - u[i]) / dx * p;
    }

    ret
}

fn backward_diff(u: &nalgebra::DVector<f64>, desc: &Descriptor) -> nalgebra::DVector<f64> {
    let n = u.len();
    let dx = desc.delta_x;
    let p = -desc.vel * desc.delta_t;

    let mut ret = nalgebra::DVector::zeros(n);

    for i in 1..n {
        ret[i] = (u[i] - u[i - 1]) / dx * p;
    }

    ret
}

fn central_diff(u: &nalgebra::DVector<f64>, desc: &Descriptor) -> nalgebra::DVector<f64> {
    let n = u.len();
    let dx = desc.delta_x;
    let p = -desc.vel * desc.delta_t;

    let mut ret = nalgebra::DVector::zeros(n);

    for i in 1..u.len() - 1 {
        let grad_1 = (u[i + 1] - u[i - 1]) / (2.0 * dx);
        ret[i] = grad_1 * p;
    }

    ret
}

fn upwind_diff(u: &nalgebra::DVector<f64>, desc: &Descriptor) -> nalgebra::DVector<f64> {
    if 0.0 <= desc.vel {
        backward_diff(u, desc)
    } else {
        forward_diff(u, desc)
    }
}

fn lax_wendroff_diff(u: &nalgebra::DVector<f64>, desc: &Descriptor) -> nalgebra::DVector<f64> {
    let n = u.len();
    let dx = desc.delta_x;
    let p = -desc.vel * desc.delta_t;

    let mut ret = nalgebra::DVector::zeros(n);

    for i in 1..u.len() - 1 {
        let grad_1 = (u[i + 1] - u[i - 1]) / (2.0 * dx);
        let grad_2 = (u[i + 1] - 2.0 * u[i] + u[i - 1]) / (2.0 * dx * dx);
        ret[i] = grad_1 * p + grad_2 * p * p;
    }

    ret
}

fn eno_diff(u: &nalgebra::DVector<f64>, desc: &Descriptor) -> nalgebra::DVector<f64> {
    let n = u.len();
    let dx = desc.delta_x;
    let p = -desc.vel * desc.delta_t;

    let mut ret = nalgebra::DVector::zeros(n);

    let d_1h = |i: usize| (u[i + 1] - u[i]) / dx;
    let d_2m = |i: usize| (d_1h(i) - d_1h(i - 1)) / (2.0 * dx);
    let d_3h = |i: usize| (d_2m(i + 1) - d_2m(i)) / (3.0 * dx);

    for i in 3..u.len() - 3 {
        let b_1 = 0.0 <= desc.vel;
        let k = if b_1 { i - 1 } else { i };

        let b_2 = 0.0 <= d_2m(k + 1).abs() - d_2m(k).abs();
        let l = if b_2 { k - 1 } else { k };

        let b_3 = 0.0 <= d_3h(l + 1).abs() - d_3h(l).abs();

        let q_1 = (u[i] - u[i - 1]) / dx;
        let q_2 = if b_2 {
            (u[i] - 2.0 * u[i - 1] + u[i - 2]) / (2.0 * dx)
        } else {
            (u[i + 1] - 2.0 * u[i] + u[i - 1]) / (2.0 * dx)
        };
        let q_3 = if b_2 && b_3 {
            (u[i] - 3.0 * u[i - 1] + 3.0 * u[i - 2] - u[i - 3]) / (3.0 * dx)
        } else if b_2 && !b_3 {
            (u[i + 1] - 3.0 * u[i] + 3.0 * u[i - 1] - u[i - 2]) / (3.0 * dx)
        } else if !b_2 && b_3 {
            (u[i + 1] - 3.0 * u[i] + 3.0 * u[i - 1] - u[i - 2]) / (-6.0 * dx)
        } else {
            (u[i + 2] - 3.0 * u[i + 1] + 3.0 * u[i] - u[i - 1]) / (-6.0 * dx)
        };

        ret[i] = (q_1 + q_2 + q_3) * p;
    }

    ret
}

fn weno_diff(u: &nalgebra::DVector<f64>, desc: &Descriptor) -> nalgebra::DVector<f64> {
    let n = u.len();
    let dx = desc.delta_x;
    let p = -desc.vel * desc.delta_t;

    let mut ret = nalgebra::DVector::zeros(n);

    let d_1l = |i: usize| (u[i] - u[i - 1]) / dx;

    for i in 3..u.len() - 3 {
        let u_1 = 1.0 / 3.0 * d_1l(i - 2) - 7.0 / 6.0 * d_1l(i - 1) + 11.0 / 6.0 * d_1l(i);
        let u_2 = -1.0 / 6.0 * d_1l(i - 1) + 5.0 / 6.0 * d_1l(i) + 1.0 / 3.0 * d_1l(i + 1);
        let u_3 = 1.0 / 3.0 * d_1l(i) + 5.0 / 6.0 * d_1l(i + 1) - 1.0 / 6.0 * d_1l(i + 2);

        let s_1 = 13.0 / 12.0 * (d_1l(i - 2) - 2.0 * d_1l(i - 1) + d_1l(i)).powi(2)
            + 1.0 / 4.0 * (d_1l(i - 2) - 4.0 * d_1l(i - 1) + 3.0 * d_1l(i)).powi(2);
        let s_2 = 13.0 / 12.0 * (d_1l(i - 1) - 2.0 * d_1l(i) + d_1l(i + 1)).powi(2)
            + 1.0 / 4.0 * (d_1l(i - 1) - d_1l(i + 1)).powi(2);
        let s_3 = 13.0 / 12.0 * (d_1l(i) - 2.0 * d_1l(i + 1) + d_1l(i + 2)).powi(2)
            + 1.0 / 4.0 * (3.0 * d_1l(i) - 4.0 * d_1l(i + 1) + d_1l(i + 2)).powi(2);

        let a_1 = 0.1 / (s_1 + 1e-6).powi(2);
        let a_2 = 0.6 / (s_2 + 1e-6).powi(2);
        let a_3 = 0.3 / (s_3 + 1e-6).powi(2);

        let w_1 = a_1 / (a_1 + a_2 + a_3);
        let w_2 = a_2 / (a_1 + a_2 + a_3);
        let w_3 = a_3 / (a_1 + a_2 + a_3);

        ret[i] = (w_1 * u_1 + w_2 * u_2 + w_3 * u_3) * p;
    }

    ret
}

fn forward_euler<F: Fn(&nalgebra::DVector<f64>, &Descriptor) -> nalgebra::DVector<f64>>(
    u: &nalgebra::DVector<f64>,
    diff_fn: F,
    desc: &Descriptor,
) -> nalgebra::DVector<f64> {
    u + diff_fn(u, desc)
}

fn rk2<F: Fn(&nalgebra::DVector<f64>, &Descriptor) -> nalgebra::DVector<f64>>(
    u: &nalgebra::DVector<f64>,
    diff_fn: F,
    desc: &Descriptor,
) -> nalgebra::DVector<f64> {
    let u_1 = u + diff_fn(u, desc);
    let u_2 = (2.0 * u + diff_fn(u, desc) + diff_fn(&u_1, desc)) / 2.0;
    u_2
}

fn rk3<F: Fn(&nalgebra::DVector<f64>, &Descriptor) -> nalgebra::DVector<f64>>(
    u: &nalgebra::DVector<f64>,
    diff_fn: F,
    desc: &Descriptor,
) -> nalgebra::DVector<f64> {
    let u_1 = u + diff_fn(u, desc);
    let u_2 = (4.0 * u + diff_fn(u, desc) + diff_fn(&u_1, desc)) / 4.0;
    let u_3 =
        (6.0 * u + diff_fn(&u_1, desc) + diff_fn(&u_1, desc) + 4.0 * diff_fn(&u_2, desc)) / 6.0;
    u_3
}

fn rk4<F: Fn(&nalgebra::DVector<f64>, &Descriptor) -> nalgebra::DVector<f64>>(
    u: &nalgebra::DVector<f64>,
    diff_fn: F,
    desc: &Descriptor,
) -> nalgebra::DVector<f64> {
    let u_1 = u + diff_fn(u, desc);

    let u_01 = (u + &u_1) / 2.0;
    let u_2 = u + diff_fn(&u_01, desc);

    let u_02 = (u + &u_2) / 2.0;
    let u_3 = u + diff_fn(&u_02, desc);

    let u_4 = u + diff_fn(&u_3, desc);

    (&u_1 + 2.0 * &u_2 + 2.0 * &u_3 + u_4) / 6.0
}

fn tvd_rk2<F: Fn(&nalgebra::DVector<f64>, &Descriptor) -> nalgebra::DVector<f64>>(
    u: &nalgebra::DVector<f64>,
    diff_fn: F,
    desc: &Descriptor,
) -> nalgebra::DVector<f64> {
    let u_1 = u + diff_fn(u, desc);
    let u_2 = (u + &u_1 + diff_fn(&u_1, desc)) / 2.0;
    u_2
}

fn tvd_rk3<F: Fn(&nalgebra::DVector<f64>, &Descriptor) -> nalgebra::DVector<f64>>(
    u: &nalgebra::DVector<f64>,
    diff_fn: F,
    desc: &Descriptor,
) -> nalgebra::DVector<f64> {
    let u_1 = u + diff_fn(u, desc);
    let u_2 = (3.0 * u + &u_1 + &diff_fn(&u_1, desc)) / 4.0;
    let u_3 = (u + 2.0 * &u_2 + 2.0 * diff_fn(&u_2, desc)) / 3.0;
    u_3
}

fn tvd_rk4<F: Fn(&nalgebra::DVector<f64>, &Descriptor) -> nalgebra::DVector<f64>>(
    u: &nalgebra::DVector<f64>,
    diff_fn: F,
    desc: &Descriptor,
) -> nalgebra::DVector<f64> {
    let u_1 = (2.0 * u + diff_fn(u, desc)) / 2.0;
    let u_2 = (2.0 * u - diff_fn(u, desc) + 2.0 * &u_1 + 2.0 * diff_fn(&u_1, desc)) / 4.0;
    let u_3 = (u - diff_fn(u, desc) + 2.0 * &u_1 - 3.0 * diff_fn(&u_1, desc)
        + 6.0 * &u_2
        + 9.0 * diff_fn(&u_2, desc))
        / 9.0;
    let u_4 =
        (2.0 * &u_1 + diff_fn(&u_1, desc) + 2.0 * &u_2 + 2.0 * &u_3 + diff_fn(&u_3, desc)) / 6.0;
    u_4
}

fn cip(
    u: &nalgebra::DVector<f64>,
    g: &nalgebra::DVector<f64>,
    desc: &Descriptor,
) -> (nalgebra::DVector<f64>, nalgebra::DVector<f64>) {
    let n = u.len();
    let dx = desc.delta_x;
    let p = -desc.vel * desc.delta_t;

    let mut ret_0 = nalgebra::DVector::zeros(n);
    let mut ret_1 = nalgebra::DVector::zeros(n);

    for i in 1..u.len() {
        let a = (g[i] + g[i - 1]) / dx.powi(2) - 2.0 * (u[i] - u[i - 1]) / dx.powi(3);
        let b = 3.0 * (u[i - 1] - u[i]) / dx.powi(2) + (2.0 * g[i] + g[i - 1]) / dx;
        let c = g[i];

        ret_0[i] = a * p.powi(3) + b * p.powi(2) + c * p + u[i];
        ret_1[i] = 3.0 * a * p.powi(2) + 2.0 * b * p + c;
    }

    (ret_0, ret_1)
}

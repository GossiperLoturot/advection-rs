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
pub enum SchemeBuffer {
    None,
    CIP { g_grid: Vec<f64> },
}

#[derive(Clone, Debug)]
pub struct Scenario {
    pub desc: Descriptor,
    u_grid: Vec<f64>,
    scheme_buffer: SchemeBuffer,
}

impl Scenario {
    pub fn new(desc: Descriptor) -> Self {
        let n = discrete(desc.bound, &desc);
        let mut u_grid = vec![0.0; n];

        let lower = discrete(0.5, &desc).min(n);
        let upper = discrete(1.0, &desc).min(n);
        for i in lower..upper {
            u_grid[i] = 1.0;
        }

        let scheme_buffer = match desc.spatial_scheme {
            SpatialScheme::CIP => {
                let g_grid = vec![0.0; n];
                SchemeBuffer::CIP { g_grid }
            }
            _ => SchemeBuffer::None,
        };

        Self {
            desc,
            u_grid,
            scheme_buffer,
        }
    }

    pub fn forward(&mut self) {
        match (self.desc.spatial_scheme, &mut self.scheme_buffer) {
            (
                SpatialScheme::Central
                | SpatialScheme::Upwind
                | SpatialScheme::LaxWendroff
                | SpatialScheme::ENO
                | SpatialScheme::WENO,
                _,
            ) => {
                let st = match self.desc.spatial_scheme {
                    SpatialScheme::Central => st_central,
                    SpatialScheme::Upwind => st_upwind,
                    SpatialScheme::LaxWendroff => st_lax_wendroff,
                    SpatialScheme::ENO => st_eno,
                    SpatialScheme::WENO => st_weno,
                    _ => unreachable!(),
                };

                let tt = match self.desc.temporal_scheme {
                    TemporalScheme::ForwardEuler => tt_forward_euler,
                    TemporalScheme::Rk2 => tt_rk2,
                    TemporalScheme::Rk3 => tt_rk3,
                    TemporalScheme::Rk4 => tt_rk4,
                    TemporalScheme::TvdRk2 => tt_tvd_rk2,
                    TemporalScheme::TvdRk3 => tt_tvd_rk3,
                    TemporalScheme::TvdRk4 => tt_tvd_rk4,
                };

                self.u_grid = tt(&self.u_grid, st, &self.desc);
            }
            (SpatialScheme::CIP, SchemeBuffer::CIP { g_grid }) => {
                let st = match self.desc.spatial_scheme {
                    SpatialScheme::CIP => st_cip,
                    _ => unreachable!(),
                };

                // Forward Euler Only
                let u = self.u_grid.clone();
                let u_suc = &mut self.u_grid;
                let g = g_grid.clone();
                let g_suc = g_grid;
                for i in 0..u.len() {
                    let (f_u, f_g) = st(i, &u, &g, &self.desc);
                    u_suc[i] = u[i] + f_u;
                    g_suc[i] = f_g;
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn show_inside(&mut self, ui: &mut egui_plot::PlotUi) {
        let points = self
            .u_grid
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

fn discrete(x: f64, desc: &Descriptor) -> usize {
    (x / desc.delta_x).round() as usize
}

fn st_forward(i: usize, u: &[f64], desc: &Descriptor) -> f64 {
    #[allow(unused_comparisons)]
    if 0 <= i && i < u.len() - 1 {
        let dx = desc.delta_x;

        let p = -desc.vel * desc.delta_t;
        (u[i + 1] - u[i]) / dx * p
    } else {
        0.0
    }
}

fn st_backward(i: usize, u: &[f64], desc: &Descriptor) -> f64 {
    if 1 <= i && i < u.len() {
        let dx = desc.delta_x;

        let p = -desc.vel * desc.delta_t;
        (u[i] - u[i - 1]) / dx * p
    } else {
        0.0
    }
}

fn st_central(i: usize, u: &[f64], desc: &Descriptor) -> f64 {
    if 1 <= i && i < u.len() - 1 {
        let dx = desc.delta_x;

        let p = -desc.vel * desc.delta_t;
        let grad_1 = (u[i + 1] - u[i - 1]) / (2.0 * dx);
        grad_1 * p
    } else {
        0.0
    }
}

fn st_upwind(i: usize, u: &[f64], desc: &Descriptor) -> f64 {
    if 0.0 <= desc.vel {
        st_backward(i, u, desc)
    } else {
        st_forward(i, u, desc)
    }
}

fn st_lax_wendroff(i: usize, u: &[f64], desc: &Descriptor) -> f64 {
    if 1 <= i && i < u.len() - 1 {
        let dx = desc.delta_x;

        let p = -desc.vel * desc.delta_t;
        let grad_1 = (u[i + 1] - u[i - 1]) / (2.0 * dx);
        let grad_2 = (u[i + 1] - 2.0 * u[i] + u[i - 1]) / (2.0 * dx * dx);
        grad_1 * p + grad_2 * p * p
    } else {
        0.0
    }
}

fn st_eno(i: usize, u: &[f64], desc: &Descriptor) -> f64 {
    if 3 <= i && i < u.len() - 3 {
        let dx = desc.delta_x;
        let d_1h = |i: usize| (u[i + 1] - u[i]) / dx;
        let d_2m = |i: usize| (d_1h(i) - d_1h(i - 1)) / (2.0 * dx);
        let d_3h = |i: usize| (d_2m(i + 1) - d_2m(i)) / (3.0 * dx);

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

        let p = -desc.vel * desc.delta_t;
        (q_1 + q_2 + q_3) * p
    } else {
        0.0
    }
}

fn st_weno(i: usize, u: &[f64], desc: &Descriptor) -> f64 {
    if 3 <= i && i < u.len() - 3 {
        let dx = desc.delta_x;
        let d_1l = |i: usize| (u[i] - u[i - 1]) / dx;

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

        let p = -desc.vel * desc.delta_t;
        (w_1 * u_1 + w_2 * u_2 + w_3 * u_3) * p
    } else {
        0.0
    }
}

fn st_cip(i: usize, u: &[f64], g: &[f64], desc: &Descriptor) -> (f64, f64) {
    if 1 <= i && i < u.len() {
        let dx = desc.delta_x;

        let a = (g[i] + g[i - 1]) / dx.powi(2) - 2.0 * (u[i] - u[i - 1]) / dx.powi(3);
        let b = 3.0 * (u[i - 1] - u[i]) / dx.powi(2) + (2.0 * g[i] + g[i - 1]) / dx;
        let c = g[i];

        let p = -desc.vel * desc.delta_t;
        let f_u = a * p.powi(3) + b * p.powi(2) + c * p;
        let f_g = 3.0 * a * p.powi(2) + 2.0 * b * p + c;
        (f_u, f_g)
    } else {
        (0.0, 0.0)
    }
}

fn tt_forward_euler<F: Fn(usize, &[f64], &Descriptor) -> f64>(
    u: &[f64],
    st: F,
    desc: &Descriptor,
) -> Vec<f64> {
    let mut u_suc = u.to_vec();

    for i in 0..u.len() {
        u_suc[i] = u[i] + st(i, u, desc);
    }

    u_suc
}

fn tt_rk2<F: Fn(usize, &[f64], &Descriptor) -> f64>(
    u_0: &[f64],
    st: F,
    desc: &Descriptor,
) -> Vec<f64> {
    let mut u_1 = u_0.to_vec();
    let mut u_2 = u_0.to_vec();

    for i in 0..u_0.len() {
        u_1[i] = u_0[i] + st(i, u_0, desc);
    }

    for i in 0..u_0.len() {
        u_2[i] = (2.0 * u_0[i] + st(i, &u_0, desc) + st(i, &u_1, desc)) / 2.0;
    }

    u_2
}

fn tt_rk3<F: Fn(usize, &[f64], &Descriptor) -> f64>(
    u_0: &[f64],
    st: F,
    desc: &Descriptor,
) -> Vec<f64> {
    let mut u_1 = u_0.to_vec();
    let mut u_2 = u_0.to_vec();
    let mut u_3 = u_0.to_vec();

    for i in 0..u_0.len() {
        u_1[i] = u_0[i] + st(i, u_0, desc);
    }

    for i in 0..u_0.len() {
        u_2[i] = (4.0 * u_0[i] + st(i, u_0, desc) + st(i, &u_1, desc)) / 4.0;
    }

    for i in 0..u_0.len() {
        u_3[i] = (6.0 * u_0[i]
            + 1.0 * st(i, u_0, desc)
            + 1.0 * st(i, &u_1, desc)
            + 4.0 * st(i, &u_2, desc))
            / 6.0;
    }

    u_3
}

fn tt_rk4<F: Fn(usize, &[f64], &Descriptor) -> f64>(
    u_0: &[f64],
    st: F,
    desc: &Descriptor,
) -> Vec<f64> {
    let mut u_1 = u_0.to_vec();
    let mut u_2 = u_0.to_vec();
    let mut u_3 = u_0.to_vec();
    let mut u_4 = u_0.to_vec();
    let mut u_01 = u_0.to_vec();
    let mut u_02 = u_0.to_vec();
    let mut u_suc = u_0.to_vec();

    for i in 0..u_0.len() {
        u_1[i] = u_0[i] + st(i, u_0, desc);
    }

    for i in 0..u_0.len() {
        u_01[i] = (u_0[i] + u_1[i]) / 2.0;
    }
    for i in 0..u_0.len() {
        u_2[i] = u_0[i] + st(i, &u_01, desc);
    }

    for i in 0..u_0.len() {
        u_02[i] = (u_0[i] + u_2[i]) / 2.0;
    }
    for i in 0..u_0.len() {
        u_3[i] = u_0[i] + st(i, &u_02, desc);
    }

    for i in 0..u_0.len() {
        u_4[i] = u_0[i] + st(i, &u_3, desc);
    }

    for i in 0..u_0.len() {
        u_suc[i] = (u_1[i] + 2.0 * u_2[i] + 2.0 * u_3[i] + u_4[i]) / 6.0;
    }

    u_suc
}

fn tt_tvd_rk2<F: Fn(usize, &[f64], &Descriptor) -> f64>(
    u_0: &[f64],
    st: F,
    desc: &Descriptor,
) -> Vec<f64> {
    let mut u_1 = u_0.to_vec();
    let mut u_2 = u_0.to_vec();

    for i in 0..u_0.len() {
        u_1[i] = u_0[i] + st(i, u_0, desc);
    }

    for i in 0..u_0.len() {
        u_2[i] = (u_0[i] + u_1[i] + st(i, &u_1, desc)) / 2.0;
    }

    u_2
}

fn tt_tvd_rk3<F: Fn(usize, &[f64], &Descriptor) -> f64>(
    u_0: &[f64],
    st: F,
    desc: &Descriptor,
) -> Vec<f64> {
    let mut u_1 = u_0.to_vec();
    let mut u_2 = u_0.to_vec();
    let mut u_3 = u_0.to_vec();

    for i in 0..u_0.len() {
        u_1[i] = u_0[i] + st(i, u_0, desc);
    }

    for i in 0..u_0.len() {
        u_2[i] = (3.0 * u_0[i] + u_1[i] + st(i, &u_1, desc)) / 4.0;
    }

    for i in 0..u_0.len() {
        u_3[i] = (u_0[i] + 2.0 * u_2[i] + 2.0 * st(i, &u_2, desc)) / 3.0;
    }

    u_3
}

fn tt_tvd_rk4<F: Fn(usize, &[f64], &Descriptor) -> f64>(
    u_0: &[f64],
    st: F,
    desc: &Descriptor,
) -> Vec<f64> {
    let mut u_1 = u_0.to_vec();
    let mut u_2 = u_0.to_vec();
    let mut u_3 = u_0.to_vec();
    let mut u_4 = u_0.to_vec();

    for i in 0..u_0.len() {
        u_1[i] = (2.0 * u_0[i] + st(i, u_0, desc)) / 2.0;
    }

    for i in 0..u_0.len() {
        u_2[i] = (2.0 * u_0[i] - st(i, u_0, desc) + 2.0 * u_1[i] + 2.0 * st(i, &u_1, desc)) / 4.0;
    }

    for i in 0..u_0.len() {
        u_3[i] = (u_0[i] - st(i, u_0, desc) + 2.0 * u_1[i] - 3.0 * st(i, &u_1, desc)
            + 6.0 * u_2[i]
            + 9.0 * st(i, &u_2, desc))
            / 9.0;
    }

    for i in 0..u_0.len() {
        u_4[i] =
            (2.0 * u_1[i] + st(i, &u_1, desc) + 2.0 * u_2[i] + 2.0 * u_3[i] + st(i, &u_3, desc))
                / 6.0;
    }

    u_4
}

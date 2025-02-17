use miniquad as mq;

mod body;
mod simulation;

pub struct State {
    egui_mq: egui_miniquad::EguiMq,
    mq_ctx: Box<dyn mq::RenderingBackend>,
    widget: body::Body,
    _thread: std::thread::JoinHandle<()>,
}

impl State {
    fn new() -> Self {
        let mut mq_ctx = mq::window::new_rendering_backend();
        let mut widget = body::Body::new();

        Self {
            egui_mq: egui_miniquad::EguiMq::new(mq_ctx.as_mut()),
            mq_ctx,
            _thread: widget.spawn_thread(),
            widget,
        }
    }
}

impl miniquad::EventHandler for State {
    fn update(&mut self) {}

    fn draw(&mut self) {
        self.mq_ctx
            .begin_default_pass(miniquad::PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        self.mq_ctx.end_render_pass();

        self.egui_mq.run(self.mq_ctx.as_mut(), |_mq_ctx, egui_ctx| {
            self.widget.show(egui_ctx);
        });

        self.egui_mq.draw(self.mq_ctx.as_mut());

        self.mq_ctx.commit_frame();
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        self.egui_mq.mouse_motion_event(x, y);
    }

    fn mouse_wheel_event(&mut self, x: f32, y: f32) {
        #[cfg(target_os = "windows")]
        let (x, y) = (x / 120.0, y / 120.0);

        self.egui_mq.mouse_wheel_event(x, y);
    }

    fn mouse_button_down_event(&mut self, button: miniquad::MouseButton, x: f32, y: f32) {
        self.egui_mq.mouse_button_down_event(button, x, y);
    }

    fn mouse_button_up_event(&mut self, button: miniquad::MouseButton, x: f32, y: f32) {
        self.egui_mq.mouse_button_up_event(button, x, y);
    }

    fn char_event(&mut self, character: char, _keymods: miniquad::KeyMods, _repeat: bool) {
        self.egui_mq.char_event(character);
    }

    fn key_down_event(
        &mut self,
        keycode: miniquad::KeyCode,
        keymods: miniquad::KeyMods,
        _repeat: bool,
    ) {
        self.egui_mq.key_down_event(keycode, keymods);
    }

    fn key_up_event(&mut self, keycode: miniquad::KeyCode, keymods: miniquad::KeyMods) {
        self.egui_mq.key_up_event(keycode, keymods);
    }
}

fn main() {
    let conf = mq::conf::Conf {
        window_title: "physics-simulation".into(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        icon: None,
        ..Default::default()
    };

    mq::start(conf, || Box::new(State::new()));
}

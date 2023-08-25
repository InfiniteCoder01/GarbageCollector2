pub mod assets;
pub mod level;
pub mod player;
use assets::*;
use speedy2d::window::WindowHelper;

fn main() {
    let handler = Game::new();
    #[cfg(target_family = "wasm")]
    speedy2d::WebCanvas::new_for_id("canvas", handler).unwrap();
    #[cfg(not(target_family = "wasm"))]
    {
        let window =
            speedy2d::Window::new_centered("Title", (640, 480)).expect("Failed to init window");
        window.run_loop(handler);
    }
}

struct Game {
    assets: Option<Assets>,
    level: level::Level,
    last_frame: std::time::Instant,
}

impl Game {
    fn new() -> Self {
        Self {
            assets: None,
            level: level::Level::new(),
            last_frame: std::time::Instant::now(),
        }
    }
}

impl speedy2d::window::WindowHandler for Game {
    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut speedy2d::Graphics2D) {
        let assets = self
            .assets
            .get_or_insert_with(|| Assets::load(graphics).expect("Failed to load assets"));
        let delta_time = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = std::time::Instant::now();
        let mut frame = Frame {
            graphics,
            delta_time,
        };

        self.level.update(&mut frame);
        helper.request_redraw();
    }
}

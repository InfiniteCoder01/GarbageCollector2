pub mod assets;
pub mod gclang;
pub mod level;
pub mod player;
use assets::*;
use level::{Level, LevelSave};
use speedy2d::font::TextLayout;
use speedy2d::window::{MouseButton, VirtualKeyCode, WindowHelper};

fn main() {
    let handler = Game::new();
    #[cfg(target_family = "wasm")]
    speedy2d::WebCanvas::new_for_id("canvas", handler).unwrap();
    #[cfg(not(target_family = "wasm"))]
    {
        let window =
            speedy2d::Window::new_centered("Garbage Collector 2: Nullptr revenge", (960, 720))
                .expect("Failed to init window");
        window.run_loop(handler);
    }
}

struct Game {
    assets: Option<Assets>,
    player: player::Player,
    levels: Vec<Level>,
    level_index: usize,

    last_frame: std::time::Instant,
    input: Input,
    size: UVec2,
    scale: f32,
}

impl Game {
    fn new() -> Self {
        macro_rules! load_level {
            ($index: expr) => {
                Level::from(
                    ron::from_str::<LevelSave>(include_str!(concat!(
                        "../Assets/Levels/",
                        stringify!($index),
                        ".ron"
                    )))
                    .expect(concat!("Failed to load level ", stringify!($index), "!")),
                )
            };
        }

        let levels = vec![
            load_level!(0),
            // Level::blank(UVec2::new(40, 10)),
        ];
        Self {
            assets: None,
            player: player::Player::new(Vec2::new(16.0, 24.0)),
            levels,
            level_index: 0,

            last_frame: std::time::Instant::now(),
            input: Input::default(),
            size: UVec2::ZERO,
            scale: 1.0,
        }
    }

    fn on_key(&mut self, key: VirtualKeyCode, state: bool) {
        #[allow(clippy::neg_multiply, clippy::identity_op)]
        match key {
            VirtualKeyCode::A => self.input.wasd.x = -1 * state as i32,
            VirtualKeyCode::D => self.input.wasd.x = 1 * state as i32,
            VirtualKeyCode::Space | VirtualKeyCode::W => self.input.wasd.y = -1 * state as i32,
            VirtualKeyCode::LShift | VirtualKeyCode::S => self.input.wasd.y = 1 * state as i32,
            _ => (),
        };
    }

    fn on_mouse(&mut self, button: MouseButton, state: bool) {
        match button {
            MouseButton::Left => self.input.mouse_left = state,
            MouseButton::Right => self.input.mouse_right = state,
            _ => (),
        };
    }
}

impl speedy2d::window::WindowHandler for Game {
    fn on_start(
        &mut self,
        _helper: &mut WindowHelper<()>,
        info: speedy2d::window::WindowStartupInfo,
    ) {
        self.size = *info.viewport_size_pixels();
    }

    fn on_resize(&mut self, _helper: &mut WindowHelper<()>, size_pixels: UVec2) {
        self.size = size_pixels;
    }

    fn on_key_down(
        &mut self,
        _helper: &mut WindowHelper<()>,
        virtual_key_code: Option<VirtualKeyCode>,
        _scancode: speedy2d::window::KeyScancode,
    ) {
        if let Some(key) = virtual_key_code {
            self.on_key(key, true);
            if key == VirtualKeyCode::Space || key == VirtualKeyCode::W {
                self.input.jump = true;
            } else if key == VirtualKeyCode::E {
                self.input.interact = true;
            } else if key == VirtualKeyCode::Escape {
                self.input.editor = !self.input.editor;
            } else if key == VirtualKeyCode::Z {
                std::fs::write(
                    format!("Assets/Levels/{}.ron", self.level_index),
                    ron::to_string(&LevelSave::from(&self.levels[self.level_index]))
                        .expect("Failed to serialize level!"),
                )
                .expect("Failed to write level!");
            }
        }
    }

    fn on_key_up(
        &mut self,
        _helper: &mut WindowHelper<()>,
        virtual_key_code: Option<VirtualKeyCode>,
        _scancode: speedy2d::window::KeyScancode,
    ) {
        if let Some(key) = virtual_key_code {
            self.on_key(key, false);
        }
    }

    fn on_mouse_move(&mut self, _helper: &mut WindowHelper<()>, position: Vec2) {
        self.input.mouse = position / self.scale;
    }

    fn on_mouse_button_down(&mut self, _helper: &mut WindowHelper<()>, button: MouseButton) {
        self.on_mouse(button, true);
    }

    fn on_mouse_button_up(&mut self, _helper: &mut WindowHelper<()>, button: MouseButton) {
        self.on_mouse(button, false);
    }

    fn on_mouse_wheel_scroll(
        &mut self,
        _helper: &mut WindowHelper<()>,
        distance: speedy2d::window::MouseScrollDistance,
    ) {
        let scroll = match distance {
            speedy2d::window::MouseScrollDistance::Lines { x: _, y, z: _ } => y as f32,
            speedy2d::window::MouseScrollDistance::Pixels { x: _, y, z: _ } => y as f32,
            speedy2d::window::MouseScrollDistance::Pages { x: _, y, z: _ } => y as f32 / 125.0,
        };

        if self.input.editor {
            let index = self.input.palette_index as f32 + scroll;
            self.input.palette_index = if index < 0.0 {
                (self.input.palette.len() as f32 + index).max(0.0) as usize
            } else {
                index as usize % self.input.palette.len()
            };
        }
    }

    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut speedy2d::Graphics2D) {
        let assets = self
            .assets
            .get_or_insert_with(|| Assets::load(graphics).expect("Failed to load assets"));

        let level = &mut self.levels[self.level_index];

        self.scale = self.size.y as f32 / level.size().y as f32 / assets.tileset.tile_size.y as f32;
        let offset = self.player.position + assets.player.tile_size.into_f32() / 2.0
            - self.size.into_f32() / self.scale / 2.0;
        let bounds = (level.size() * assets.tileset.tile_size).into_f32()
            - self.size.into_f32() / self.scale;
        let mut camera = Camera {
            graphics,
            offset: Vec2::new(offset.x.clamp(0.0, bounds.x), offset.y.clamp(0.0, bounds.y)),
            scale: self.scale,
        };

        let delta_time = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = std::time::Instant::now();

        level.update(assets, &mut camera, delta_time);
        self.player
            .update(assets, level, &mut camera, &mut self.input, delta_time);

        if self.input.editor {
            graphics.draw_text(
                Vec2::new(10.0, 10.0),
                Color::BLUE,
                &assets.font.layout_text("Editor", 24.0, Default::default()),
            )
        }

        if let Some(program) = &mut self.input.terminal {
            use gclang::Value;
            get_screen_buffer(&mut self.input.scopes);

            let mut library = gclang::Library::with_std();
            library_function!(library += print (scopes, args) {
                let output = args
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                get_screen_buffer(scopes).push_str(&output);
                Ok(Value::Unit)
            });
            program.eval(&mut self.input.scopes, &mut library).unwrap();

            // * Draw terminal
            let size = assets.terminal.size().into_f32()
                * (self.size.y as f32 / 1.2 / assets.terminal.size().y as f32);
            let tl = (self.size.into_f32() - size) / 2.0;
            graphics.draw_rectangle_image(
                speedy2d::shape::Rectangle::new(tl, tl + size),
                &assets.terminal,
            );

            let (screen_width, screen_height) = (54, 28);
            let screen = get_screen_buffer(&mut self.input.scopes);

            let mut line_width = 0;
            let mut new_screen = String::new();
            for ch in screen.chars() {
                if line_width >= screen_width {
                    line_width = 0;
                    if ch != '\n' {
                        new_screen.push('\n');
                    }
                }
                if ch == '\n' {
                    line_width = 0;
                } else {
                    line_width += 1;
                }
                new_screen.push(ch);
            }
            *screen = String::new();
            for (index, line) in new_screen.split_inclusive('\n').rev().enumerate() {
                if index >= screen_height {
                    break;
                }
                screen.insert_str(0, line)
            }

            graphics.draw_text(
                tl + size / Vec2::new(17.0, 14.0),
                Color::GREEN,
                &assets
                    .font
                    .layout_text(screen, (size.y - 26.0) / 30.0, Default::default()),
            )
        }

        self.input.jump = false;
        self.input.interact = false;
        helper.request_redraw();
    }
}

fn get_screen_buffer(scopes: &mut gclang::Scopes) -> &mut String {
    let screen = scopes.get_global_or_insert(
        "screen_buffer",
        gclang::Value::String(String::from("Net Terminal V1.0\n")),
    );

    if !matches!(screen, gclang::Value::String(_)) {
        *screen = gclang::Value::String(String::from("Refreshing buffer, it was of wrong type.\n"));
    }

    match screen {
        gclang::Value::String(screen) => screen,
        _ => unreachable!(),
    }
}

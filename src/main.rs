pub mod assets;
pub mod gclang;
pub mod level;
pub mod player;
use assets::*;
use level::{Level, LevelSave};
use speedy2d::font::{TextLayout, TextOptions};
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

    last_frame: instant::Instant,
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
            load_level!(1),
            load_level!(2),
            load_level!(3),
            // Level::blank(UVec2::new(20, 10)),
        ];
        Self {
            assets: None,
            player: player::Player::new(Vec2::new(16.0, 24.0)),
            levels,

            last_frame: instant::Instant::now(),
            input: Input::default(),
            size: UVec2::ZERO,
            scale: 1.0,
        }
    }

    fn on_key(&mut self, key: VirtualKeyCode, state: bool) {
        #[allow(clippy::neg_multiply, clippy::identity_op)]
        match key {
            VirtualKeyCode::A | VirtualKeyCode::Left => self.input.wasd.x = -1 * state as i32,
            VirtualKeyCode::D | VirtualKeyCode::Right => self.input.wasd.x = 1 * state as i32,
            VirtualKeyCode::Space | VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.input.wasd.y = -1 * state as i32
            }
            VirtualKeyCode::LShift | VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.input.wasd.y = 1 * state as i32
            }
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
            match key {
                VirtualKeyCode::Left => self.input.arrows.x = -1,
                VirtualKeyCode::Right => self.input.arrows.x = 1,
                VirtualKeyCode::Up => self.input.arrows.y = -1,
                VirtualKeyCode::Down => self.input.arrows.y = 1,
                VirtualKeyCode::E => self.input.interact = true,
                // VirtualKeyCode::Escape => self.input.editor = !self.input.editor,
                // VirtualKeyCode::Z => {
                //     std::fs::write(
                //         format!("Assets/Levels/{}.ron", self.input.index),
                //         ron::to_string(&LevelSave::from(&self.levels[self.input.index]))
                //             .expect("Failed to serialize level!"),
                //     )
                //     .expect("Failed to write level!");
                // }
                _ => (),
            }
            if key == VirtualKeyCode::Space || key == VirtualKeyCode::W || key == VirtualKeyCode::Up
            {
                self.input.jump = true;
            }
        }
    }

    fn on_keyboard_char(&mut self, _helper: &mut WindowHelper<()>, unicode_codepoint: char) {
        if unicode_codepoint != '\x1b' {
            self.input.typed_text.push(match unicode_codepoint {
                '\r' => '\n',
                codepoint => codepoint,
            });
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
        } else if let Some(terminal) = &mut self.input.terminal {
            terminal.scroll = (terminal.scroll as f32 + scroll).max(0.0) as _;
        }
    }

    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut speedy2d::Graphics2D) {
        let assets = self
            .assets
            .get_or_insert_with(|| Assets::load(graphics).expect("Failed to load assets"));

        let mut level = &mut self.levels[self.input.index];

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
        self.last_frame = instant::Instant::now();

        level.update(assets, &mut self.input, &mut camera, &self.player, delta_time);
        self.player
            .update(assets, &mut self.input, level, &mut camera, delta_time);

        if self.input.editor {
            graphics.draw_text(
                Vec2::new(10.0, 10.0),
                Color::BLUE,
                &assets.font.layout_text("Editor", 24.0, Default::default()),
            )
        }

        if let Some(next_level) = self.input.next_index {
            self.input.index = next_level;
            self.input.next_index = None;
            level = &mut self.levels[self.input.index];
            if self.player.position.x >= level.size().x as f32 / 2.0 {
                self.player.position.x = 16.0;
            } else {
                self.player.position.x =
                    (level.size().x - 2) as f32 * assets.tileset.tile_size.x as f32;
            }
        }

        if let Some(terminal) = &mut self.input.terminal {
            if let Some(log) = self.input.scopes.get_path(vec!["home", "log"]) {
                *log = gclang::Value::String(assets.logs[self.input.index].to_owned());
            }

            let mut should_exit = false;
            {
                use gclang::Value;
                get_screen_buffer(&mut self.input.scopes);
                let border = UVec2::from(13);
                let screen_height = 30usize;
                let screen_width =
                    (((assets.terminal.size().x - border.x * 2) * screen_height as u32
                        / (assets.terminal.size().y - border.y * 2)) as f32
                        / assets
                            .font
                            .layout_text("#", 1.0, Default::default())
                            .width()) as usize;

                let mut library = gclang::Library::with_std();
                library_function!(library += print (scopes, args) {
                    let output = args
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ");
                    get_screen_buffer(scopes).push_str(&output);
                    gclang::Ok(Value::Unit)
                });
                library_function!(library += println (scopes, args) {
                    let output = args
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ");
                    let screen = get_screen_buffer(scopes);
                    screen.push_str(&output);
                    screen.push('\n');
                    gclang::Ok(Value::Unit)
                });
                library_function!(library += input (_scopes, args) {
                    gclang::ensure!(args.is_empty(), "input() was not ment to be used with arguments!");
                    gclang::Ok(Value::String(self.input.typed_text.clone()))
                });
                library_function!(library += delta_time (_scopes, args) {
                    gclang::ensure!(args.is_empty(), "delta_time() was not ment to be used with arguments!");
                    gclang::Ok(Value::Int((delta_time * 1000.0) as _))
                });
                library_function!(library += level_index (_scopes, args) {
                    gclang::ensure!(args.is_empty(), "level_index() was not ment to be used with arguments!");
                    gclang::Ok(Value::Int(self.input.index as _))
                });
                library_function!(library += arrows_x (_scopes, args) {
                    gclang::ensure!(args.is_empty(), "arrows_x() was not ment to be used with arguments!");
                    gclang::Ok(Value::Int(self.input.arrows.x))
                });
                library_function!(library += arrows_y (_scopes, args) {
                    gclang::ensure!(args.is_empty(), "arrows_x() was not ment to be used with arguments!");
                    gclang::Ok(Value::Int(self.input.arrows.y))
                });
                library_function!(library += screen_width (_scopes, args) {
                    gclang::ensure!(args.is_empty(), "screen_width() was not ment to be used with arguments!");
                    gclang::Ok(Value::Int(screen_width as _))
                });
                library_function!(library += screen_height (_scopes, args) {
                    gclang::ensure!(args.is_empty(), "screen_height() was not ment to be used with arguments!");
                    gclang::Ok(Value::Int(screen_height as _))
                });
                library_function!(library += exit (_scopes, args) {
                    gclang::ensure!(args.is_empty(), "exit() was not ment to be used with arguments!");
                    should_exit = true;
                    gclang::Ok(Value::Unit)
                });
                if let Err(error) = terminal.program.eval(&mut self.input.scopes, &mut library) {
                    if let Some(error) = match error {
                        gclang::Exception::Error(error) => Some(error.to_string()),
                        gclang::Exception::Effect(gclang::Effect {
                            effect, handler, ..
                        })
                        | gclang::Exception::EffectUnwind(effect, handler, _) => Some(format!(
                            "Unhandled effect '{}' (handler '{}')!",
                            effect, handler
                        )),
                        gclang::Exception::Resume(_) => {
                            Some(String::from("Internal error: Resume lost path!"))
                        }
                        gclang::Exception::Return(_) => None,
                    } {
                        let screen = get_screen_buffer(&mut self.input.scopes);
                        screen.push_str("\x1bff0000");
                        screen.push_str(&error);
                        screen.push_str("\x18\n");
                    }
                }

                // * Draw terminal
                let scale = self.size.y as f32 / 1.2 / assets.terminal.size().y as f32;
                let size = assets.terminal.size().into_f32() * scale;
                let tl = (self.size.into_f32() - size) / 2.0;
                graphics.draw_rectangle_image(
                    speedy2d::shape::Rectangle::new(tl, tl + size),
                    &assets.terminal,
                );

                let border = border.into_f32() * scale;
                let tl = tl + border;
                let line_height = (size.y - border.y * 2.0) / screen_height as f32;
                let screen = get_screen_buffer(&mut self.input.scopes);
                let mut cursor = tl;
                let mut color = None;
                let mut background = None;
                let line_count = screen.matches('\n').count() + 1;
                terminal.scroll = terminal
                    .scroll
                    .min(line_count.max(screen_height) - screen_height);
                for (index, line) in screen.split('\n').enumerate() {
                    let visible = (terminal.scroll..=terminal.scroll + screen_height)
                        .contains(&(line_count - index));
                    let mut sections = Vec::new();
                    let mut last_index = 0;
                    for (index, escape) in line.match_indices(['\x1b', '\x1c', '\x18', '\x19']) {
                        if visible {
                            sections.push((color, background, &line[last_index..index]));
                        }
                        if escape == "\x1b" || escape == "\x1c" {
                            if line.len() <= index + 6 {
                                continue;
                            }
                            if let Result::Ok(color_hex) =
                                u32::from_str_radix(&line[index + 1..=index + 6], 16)
                            {
                                if escape == "\x1b" {
                                    color = Some(Color::from_hex_rgb(color_hex));
                                } else {
                                    background = Some(Color::from_hex_rgb(color_hex));
                                }
                            }
                            last_index = index + 7;
                        } else {
                            if escape == "\x18" {
                                color = None;
                            } else {
                                background = None;
                            }
                            last_index = index + 1;
                        }
                    }
                    if visible {
                        sections.push((color, background, &line[last_index..]));
                        for (color, background, section) in sections {
                            let section = &assets.font.layout_text(
                                section,
                                line_height,
                                TextOptions::default().with_trim_each_line(false),
                            );
                            if let Some(background) = background {
                                graphics.draw_rectangle(
                                    speedy2d::shape::Rectangle::new(
                                        cursor,
                                        cursor + section.size(),
                                    ),
                                    background,
                                );
                            }
                            graphics.draw_text(cursor, color.unwrap_or(Color::GREEN), section);
                            cursor.x += section.width();
                        }
                        cursor.y += line_height;
                        cursor.x = tl.x;
                    }
                }
            }
            if should_exit {
                self.input.terminal = None;
            }
        }

        self.input.jump = false;
        self.input.interact = false;
        self.input.arrows = IVec2::ZERO;
        self.input.typed_text.clear();
        helper.request_redraw();
    }
}

fn get_screen_buffer(scopes: &mut gclang::Scopes) -> &mut String {
    let screen = scopes.get_global_or_insert(
        "screen_buffer",
        gclang::Value::String(String::from("Net Terminal V1.0\nCtrl+Q to exit\nType \"edit /home/log\" to view logs.\n")),
    );

    if !matches!(screen, gclang::Value::String(_)) {
        *screen = gclang::Value::String(String::from(
            "Refreshing buffer, it was of wrong type (probably internal error).\n",
        ));
    }

    match screen {
        gclang::Value::String(screen) => screen,
        _ => unreachable!(),
    }
}

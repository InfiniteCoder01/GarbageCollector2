use crate::assets::*;
use crate::level::{Level, Tile};

pub struct Player {
    pub position: Vec2,
    velocity: Vec2,
    jumps: u32,
    pub max_jumps: u32,
}

impl Player {
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            velocity: Vec2::ZERO,
            jumps: 0,
            max_jumps: 2,
        }
    }

    fn collides(&mut self, assets: &Assets, level: &Level, input: &mut Input) -> bool {
        let filewall_whitelist = match input.scopes.get_path(vec!["firewall", "whitelist"]) {
            Some(crate::gclang::Value::String(whitelist)) => whitelist
                .split('\n')
                .any(|line| line.eq_ignore_ascii_case("Garbage Collector")),
            _ => false,
        };
        let private_key = match input.scopes.get_path(vec!["rsa", "keys"]) {
            Some(crate::gclang::Value::String(whitelist)) => whitelist
                .split('\n')
                .any(|line| line.eq_ignore_ascii_case("SGkh")),
            _ => false,
        };

        let tl = (self.position + 1.0) / assets.tileset.tile_size.into_f32();
        let tl = Vec2::new(tl.x.floor(), tl.y.floor()).into_i32();
        let br = ((self.position - 1.0) + assets.player.tile_size.into_f32())
            / assets.tileset.tile_size.into_f32();
        let br = Vec2::new(br.x.ceil(), br.y.ceil()).into_i32();
        for y in tl.y..br.y {
            for x in tl.x..br.x {
                if let Some(tile) = level.tile(IVec2::new(x, y)) {
                    match tile {
                        Tile::Ground | Tile::Block => return true,
                        Tile::Firewall => {
                            if !filewall_whitelist {
                                self.velocity.x = (self.position.x
                                    - x as f32 * assets.tileset.tile_size.x as f32)
                                    .signum()
                                    * 500.0;
                            }
                        }
                        Tile::Private => {
                            if !private_key {
                                self.velocity.x = (self.position.x
                                    - x as f32 * assets.tileset.tile_size.x as f32)
                                    .signum()
                                    * 500.0;
                            }
                        }
                        Tile::Terminal => {
                            if input.interact {
                                input.terminal = Some(Terminal::new(crate::gclang::gcsh()));
                                input.interact = false;
                            }
                        }
                        Tile::Port => {
                            if input.interact && level.is_lit(input) {
                                if x >= level.size().x as i32 / 2 {
                                    if y == 0 {
                                        input.next_index = Some(input.index + 2);
                                    } else {
                                        input.next_index = Some(input.index + 1);
                                    }
                                } else {
                                    input.next_index = Some(input.index - 1);
                                }
                                input.interact = false;
                            }
                        }
                        _ => (),
                    }
                } else {
                    return true;
                }
            }
        }
        false
    }

    pub(crate) fn update(
        &mut self,
        assets: &Assets,
        input: &mut Input,
        level: &mut Level,
        camera: &mut Camera,
        delta_time: f32,
    ) {
        if input.terminal.is_none() {
            if input.editor {
                self.velocity = input.wasd.into_f32() * 16.0 * 10.0;
                let hover_tile =
                    (input.mouse + camera.offset).into_i32() / assets.tileset.tile_size.into_i32();
                if let Some(tile) = level.tile_mut(hover_tile) {
                    if input.mouse_right {
                        *tile = Tile::Empty;
                    } else if input.mouse_left {
                        *tile = input.palette[input.palette_index];
                    }
                    if !input.mouse_right {
                        level.draw_tile(
                            assets,
                            input,
                            camera,
                            hover_tile.into_u32(),
                            input.palette[input.palette_index],
                        );
                    }
                }
            } else {
                let target_velocity = input.wasd.x as f32 * 16.0 * 6.0;
                self.velocity.x += (target_velocity - self.velocity.x) * delta_time * 7.0;
                self.velocity.y += 1000.0 * delta_time;
                if input.jump && self.jumps > 0 {
                    self.velocity.y = -300.0;
                    self.jumps -= 1;
                }
            }

            self.integrate(assets, level, input, Vec2::new_x(1.0), delta_time);
            self.integrate(assets, level, input, Vec2::new_y(1.0), delta_time);
        }
        assets.player.draw_tile(camera, self.position, UVec2::ZERO);
    }

    fn integrate(
        &mut self,
        assets: &Assets,
        level: &Level,
        input: &mut Input,
        dir: Vec2,
        delta_time: f32,
    ) {
        let motion = self.velocity * dir * delta_time;
        self.position += motion;
        if self.collides(assets, level, input) {
            self.velocity *= Vec2::new(1.0, 1.0) - dir;
            if motion.y.signum() > 0.0 {
                self.jumps = self.max_jumps;
            }
            while self.collides(assets, level, input) {
                self.position -= motion / (motion.x + motion.y).abs() * 0.05;
            }
        }
    }
}

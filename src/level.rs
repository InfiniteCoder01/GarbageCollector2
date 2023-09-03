use crate::assets::*;
use bidivec::*;
use rand::Rng;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tile {
    #[default]
    Empty,
    Ground,
    Table,
    Terminal,
    Block,
    Port,
    Firewall,
    Private,
}

pub struct Level {
    tilemap: BidiArray<Tile>,
    particles: Vec<Particle>,
}

struct Particle {
    position: Vec2,
    velocity: Vec2,
    time: f32,
}

impl Particle {
    fn new(position: Vec2, velocity: Vec2) -> Self {
        Self {
            position,
            velocity,
            time: 1.0,
        }
    }

    fn update(&mut self, camera: &mut Camera, delta_time: f32, away: Option<Vec2>) {
        let mut velocity = self.velocity;
        if let Some(away) = away {
            let vector = self.position - away;
            velocity += vector.normalize().unwrap_or(vector)
                * (1.0 - vector.magnitude() / 15.0).clamp(0.0, 1.0)
                * 300.0;
        }
        self.position += velocity * delta_time;
        self.time -= delta_time;
        camera.graphics.draw_circle(
            (self.position - camera.offset) * camera.scale,
            camera.scale * 0.3,
            Color::from_rgb(
                self.time.clamp(0.0, 0.5) * 2.0,
                (0.5 - (0.5 - self.time).abs()).max(0.0) * 2.0,
                0.0,
            ),
        )
    }
}

impl Level {
    pub fn blank(size: UVec2) -> Self {
        let mut tilemap = bidiarray![Tile::Empty; size.x as usize, size.y as usize];
        tilemap
            .iter_mut()
            .on_rect(&BidiRect::new(0, size.y as usize - 2, size.x as usize, 2))
            .for_each(|tile| *tile = Tile::Ground);
        Self {
            tilemap,
            particles: Vec::new(),
        }
    }

    pub fn tile(&self, pos: IVec2) -> Option<Tile> {
        if pos.x < 0 || pos.y < 0 {
            return None;
        }
        self.tilemap.get(pos.x as _, pos.y as _).copied()
    }

    pub fn tile_mut(&mut self, pos: IVec2) -> Option<&mut Tile> {
        if pos.x < 0 || pos.y < 0 {
            return None;
        }
        self.tilemap.get_mut(pos.x as _, pos.y as _)
    }

    pub fn is_lit(&self, input: &mut Input) -> bool {
        match input
            .scopes
            .get_global_or_insert("network_service", crate::gclang::Value::Bool(false))
        {
            crate::gclang::Value::Bool(network_service) => *network_service,
            _ => false,
        }
    }

    pub fn size(&self) -> UVec2 {
        UVec2::new(self.tilemap.width() as _, self.tilemap.height() as _)
    }

    pub fn draw_tile(
        &mut self,
        assets: &Assets,
        input: &mut Input,
        camera: &mut Camera,
        tile_pos: UVec2,
        tile: Tile,
    ) {
        let screen_pos = tile_pos.into_f32() * assets.tileset.tile_size.into_f32();
        if tile == Tile::Ground {
            assets
                .tileset
                .draw_patch(camera, screen_pos, UVec2::new(0, 0), |offset| {
                    self.tile(tile_pos.into_i32() + offset) != Some(tile)
                });
        } else if tile == Tile::Table {
            let left = self.tile(tile_pos.into_i32() - IVec2::new_x(1)) != Some(tile);
            let right = self.tile(tile_pos.into_i32() + IVec2::new_x(1)) != Some(tile);
            let offset = 1 + right as u32 - left as u32;
            assets
                .tileset
                .draw_tile(camera, screen_pos, UVec2::new(offset, 2));
        } else if tile == Tile::Firewall {
            let mut rng = rand::thread_rng();
            for _ in 0..100 {
                self.particles.push(Particle::new(
                    screen_pos
                        + Vec2::new(
                            rng.gen_range(0..assets.tileset.tile_size.x * 5) as f32 / 5.0,
                            rng.gen_range(0..assets.tileset.tile_size.y * 5) as f32 / 5.0,
                        ),
                    Vec2::new(rng.gen_range(-1..=1) as _, -10.0),
                ));
            }
        } else {
            let (tile, size) = match tile {
                Tile::Terminal => (UVec2::new(1, 1), UVec2::new(1, 1)),
                Tile::Block => (UVec2::new(4, 0), UVec2::new(1, 1)),
                Tile::Port => (
                    UVec2::new(3 + self.is_lit(input) as u32, 1),
                    UVec2::new(1, 2),
                ),
                _ => (UVec2::ZERO, UVec2::ZERO),
            };
            for y in 0..size.y {
                for x in 0..size.x {
                    assets.tileset.draw_tile(
                        camera,
                        screen_pos
                            + UVec2::new(x, y).into_f32() * assets.tileset.tile_size.into_f32(),
                        tile + UVec2::new(x, y),
                    );
                }
            }
        }
    }

    pub(crate) fn update(
        &mut self,
        assets: &Assets,
        input: &mut Input,
        camera: &mut Camera,
        player: &crate::player::Player,
        delta_time: f32,
    ) {
        camera.graphics.clear_screen(Color::from_hex_rgb(0x87CEEB));

        for y in 0..self.size().y {
            for x in 0..self.size().x {
                let tile_pos = UVec2::new(x as _, y as _);
                if let Some(tile) = self.tile(IVec2::new(x as _, y as _)) {
                    self.draw_tile(assets, input, camera, tile_pos, tile);
                }
            }
        }

        let filewall_whitelist = match input.scopes.get_path(vec!["firewall", "whitelist"]) {
            Some(crate::gclang::Value::String(whitelist)) => whitelist
                .split('\n')
                .any(|line| line.eq_ignore_ascii_case("Garbage Collector")),
            _ => false,
        };
        for particle in &mut self.particles {
            particle.update(
                camera,
                delta_time,
                if filewall_whitelist {
                    Some(player.position + assets.player.tile_size.into_f32() / 2.0)
                } else {
                    None
                },
            );
        }
        self.particles.retain(|particle| particle.time > 0.0);
    }
}

#[derive(Serialize, Deserialize)]
pub struct LevelSave {
    size: (usize, usize),
    tilemap: Vec<Tile>,
}

impl From<&Level> for LevelSave {
    fn from(level: &Level) -> Self {
        Self {
            size: level.tilemap.size(),
            tilemap: level.tilemap.iter().copied().collect(),
        }
    }
}

impl From<LevelSave> for Level {
    fn from(mut save: LevelSave) -> Self {
        assert_eq!(save.size.0 * save.size.1, save.tilemap.len());
        Self {
            tilemap: BidiArray::from_iterator(save.tilemap.drain(..), save.size.0)
                .expect("Failed to construct tilemap!"),
            particles: Vec::new(),
        }
    }
}

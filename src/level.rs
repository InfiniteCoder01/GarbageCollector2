use crate::assets::*;
use bidivec::*;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tile {
    #[default]
    Empty,
    Ground,
    Table,
    Terminal,
}

pub struct Level {
    tilemap: BidiArray<Tile>,
}

impl Level {
    pub fn blank(size: UVec2) -> Self {
        let mut tilemap = bidiarray![Tile::Empty; size.x as usize, size.y as usize];
        tilemap
            .iter_mut()
            .on_rect(&BidiRect::new(0, size.y as usize - 2, size.x as usize, 2))
            .for_each(|tile| *tile = Tile::Ground);
        Self { tilemap }
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

    pub fn size(&self) -> UVec2 {
        UVec2::new(self.tilemap.width() as _, self.tilemap.height() as _)
    }

    pub fn draw_tile(&self, camera: &mut Camera, assets: &Assets, tile_pos: UVec2, tile: Tile) {
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
        } else {
            match tile {
                Tile::Terminal => assets
                    .tileset
                    .draw_tile(camera, screen_pos, UVec2::new(1, 1)),
                _ => (),
            }
        }
    }

    pub(crate) fn update(&mut self, assets: &Assets, camera: &mut Camera, _delta_time: f32) {
        camera.graphics.clear_screen(Color::from_hex_rgb(0x87CEEB));

        for y in 0..self.size().y {
            for x in 0..self.size().x {
                let tile_pos = UVec2::new(x as _, y as _);
                if let Some(tile) = self.tile(IVec2::new(x as _, y as _)) {
                    self.draw_tile(camera, assets, tile_pos, tile);
                }
            }
        }
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
        }
    }
}

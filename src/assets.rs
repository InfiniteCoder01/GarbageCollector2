pub use anyhow::*;
pub use serde::{Deserialize, Serialize};
pub use speedy2d::color::Color;
pub use speedy2d::dimen::*;
use speedy2d::image::ImageHandle;
use speedy2d::shape::Rectangle;

pub struct Assets {
    pub font: speedy2d::font::Font,
    pub logs: Vec<&'static str>,

    pub tileset: Atlas,
    pub player: Atlas,
    pub terminal: ImageHandle,
}

impl Assets {
    pub(crate) fn load(graphics: &mut speedy2d::Graphics2D) -> Result<Self> {
        macro_rules! load_texture {
            ($name: ident) => {
                graphics
                    .create_image_from_file_bytes(
                        Some(speedy2d::image::ImageFileFormat::PNG),
                        speedy2d::image::ImageSmoothingMode::NearestNeighbor,
                        std::io::Cursor::new(include_bytes!(concat!(
                            "../Assets/Textures/",
                            stringify!($name),
                            ".png"
                        ))),
                    )
                    .map_err(|err| anyhow!(err.to_string()))?
            };
            ($name: ident, $tile_size: expr) => {
                Atlas {
                    image: load_texture!($name),
                    tile_size: UVec2::from($tile_size),
                }
            };
        }

        Ok(Self {
            font: speedy2d::font::Font::new(include_bytes!("../Assets/JoystixMonospace.ttf"))
                .map_err(|err| anyhow!(err.to_string()))?,
            logs: vec![
                "Nullptr exception!\n[systemctl] Network service was stopped because of OOM,\nrun \"systemctl restart network\" to restart it.\nUser with name \"Garbage Collector\" logged in.",
                "*4 log lines skipped*\n[firewall] User \"Garbage Collector\" is not in whitelist.\nConsider adding their name to /firewall/whitelist",
                "*6 log lines skipped*\n[rsa-service] RSA key leaked: \"SGkh\"!",
                concat!(
                    "*7 log lines skipped*\n",
                    "Dear player!\n",
                    "Unfortunately, I didn't manage to get this game\n",
                    "done. I spend too much time making a\n",
                    "langage for it.\n",
                    "You can study it by viewing files in /bin\n",
                    "with edit. You can write something in it using\n",
                    "\"edit /bin/myprogram\", type some code and\n",
                    "write \"someprogram\"."
                ),
            ],

            tileset: load_texture!(Tileset, (16, 16)),
            player: load_texture!(Player, (16, 24)),
            terminal: load_texture!(Terminal),
        })
    }
}

pub struct Atlas {
    pub image: ImageHandle,
    pub tile_size: UVec2,
}

impl Atlas {
    pub fn draw_tile(&self, camera: &mut Camera, position: Vec2, tile: UVec2) {
        let position = (position - camera.offset) * camera.scale;
        let size = self.tile_size.into_f32() * camera.scale;
        let size = Vec2::new(size.x.ceil(), size.y.ceil());
        let src_pos = tile * self.tile_size;
        let src_pos = src_pos.into_f32() / self.image.size().into_f32();
        camera.graphics.draw_rectangle_image_subset_tinted(
            Rectangle::new(position, position + size),
            Color::WHITE,
            Rectangle::new(
                src_pos,
                src_pos + self.tile_size.into_f32() / self.image.size().into_f32(),
            ),
            &self.image,
        );
    }

    pub fn draw_patch(
        &self,
        camera: &mut Camera,
        position: Vec2,
        tile: UVec2,
        edge: impl Fn(IVec2) -> bool,
    ) {
        for direction in [
            UVec2::new(0, 0),
            UVec2::new(0, 1),
            UVec2::new(1, 0),
            UVec2::new(1, 1),
        ] {
            let offset = {
                let direction = direction.into_i32() * 2 - 1;
                edge(direction * IVec2::new_y(1)) as u32
                    | (edge(direction * IVec2::new_x(1)) as u32 * 2)
            };

            let position = (position - camera.offset
                + direction.into_f32() * self.tile_size.into_f32() / 2.0)
                * camera.scale;
            let size = self.tile_size.into_f32() * camera.scale / 2.0;
            let size = Vec2::new(size.x.ceil(), size.y.ceil());
            let src_pos =
                (tile + UVec2::new_x(offset)) * self.tile_size + direction * self.tile_size / 2;
            let src_size = self.tile_size / 2;
            camera.graphics.draw_rectangle_image_subset_tinted(
                Rectangle::new(position, position + size),
                Color::WHITE,
                Rectangle::new(
                    src_pos.into_f32() / self.image.size().into_f32(),
                    (src_pos + src_size).into_f32() / self.image.size().into_f32(),
                ),
                &self.image,
            )
        }
    }

    pub fn size(&self) -> UVec2 {
        self.image.size() / self.tile_size
    }
}

// * ------------------------------------- Frame ------------------------------------ * //
pub struct Camera<'a> {
    pub graphics: &'a mut speedy2d::Graphics2D,
    pub offset: Vec2,
    pub scale: f32,
}

pub struct Input {
    pub wasd: IVec2,
    pub arrows: IVec2,
    pub jump: bool,
    pub interact: bool,
    pub ctrl: bool,
    pub alt: bool,

    pub mouse: Vec2,
    pub mouse_left: bool,
    pub mouse_right: bool,

    pub index: usize,
    pub next_index: Option<usize>,
    pub editor: bool,
    pub palette: Vec<crate::level::Tile>,
    pub palette_index: usize,

    pub typed_text: String,
    pub scopes: crate::gclang::Scopes,
    pub terminal: Option<Terminal>,
}

pub struct Terminal {
    pub program: crate::gclang::Program,
    pub scroll: usize,
}

impl Terminal {
    pub fn new(program: crate::gclang::Program) -> Self {
        Self { program, scroll: 0 }
    }
}

impl Default for Input {
    fn default() -> Self {
        use crate::level::Tile;

        Self {
            wasd: IVec2::ZERO,
            arrows: IVec2::ZERO,
            jump: false,
            interact: false,
            ctrl: false,
            alt: false,

            mouse: Vec2::ZERO,
            mouse_left: false,
            mouse_right: false,

            index: 0,
            next_index: None,
            editor: false,
            palette: vec![
                Tile::Ground,
                Tile::Table,
                Tile::Terminal,
                Tile::Block,
                Tile::Port,
                Tile::Firewall,
                Tile::Private,
            ],
            palette_index: 0,

            typed_text: String::new(),
            scopes: crate::gclang::Scopes::default(),
            terminal: None,
        }
    }
}

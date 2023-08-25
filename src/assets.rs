pub use anyhow::*;
pub use speedy2d::dimen::*;
use speedy2d::image::ImageHandle;

pub struct Assets {
    pub test_texture: ImageHandle,
}

impl Assets {
    pub(crate) fn load(graphics: &mut speedy2d::Graphics2D) -> Result<Self> {
        macro_rules! load_image {
            ($name: ident) => {
                graphics
                    .create_image_from_file_bytes(
                        Some(speedy2d::image::ImageFileFormat::PNG),
                        speedy2d::image::ImageSmoothingMode::NearestNeighbor,
                        std::io::Cursor::new(include_bytes!(concat!(
                            "../Assets/",
                            stringify!($name),
                            ".png"
                        ))),
                    )
                    .map_err(|err| anyhow!(err.to_string()))?
            };
        }

        Ok(Self {
            test_texture: load_image!(Test),
        })
    }
}

pub struct Frame<'a> {
    pub graphics: &'a mut speedy2d::Graphics2D,
    pub delta_time: f32,
}

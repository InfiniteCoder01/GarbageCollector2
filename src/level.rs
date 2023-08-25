use crate::assets::*;
use crate::player::Player;

pub struct Level {
    pub size: UVec2,
    pub player: Player,
}

impl Level {
    pub fn new() -> Self {
        Self {
            size: UVec2::ZERO,
            player: Player::new(Vec2::ZERO),
        }
    }

    pub(crate) fn update(&mut self, frame: &mut Frame) {
        frame.graphics.clear_screen(speedy2d::color::Color::from_hex_rgb(0x87CEEB));
        self.player.update(frame);
    }
}

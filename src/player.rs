use crate::assets::*;

pub struct Player {
    position: Vec2,
    acceleration: Vec2,
}

impl Player {
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            acceleration: Vec2::ZERO,
        }
    }

    pub fn update(&mut self, frame: &mut Frame) {
        self.position += self.acceleration * frame.delta_time;
        frame.graphics.draw_rectangle(
            speedy2d::shape::Rectangle::new(self.position, Vec2::new(60.0, 100.0)),
            speedy2d::color::Color::RED,
        );
    }
}

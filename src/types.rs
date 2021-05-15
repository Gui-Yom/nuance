use crevice::std430::{AsStd430, Std430};
use mint::Vector2;

/// The globals we pass to the fragment shader
/// aligned to 32bit words
#[derive(AsStd430)]
pub struct Globals {
    /// Window resolution
    pub resolution: Vector2<u32>,
    /// Mouse pos
    pub mouse: Vector2<u32>,
    /// Mouse wheel
    pub mouse_wheel: f32,
    /// Draw area width/height ratio
    pub ratio: f32,
    /// Current running time in sec
    pub time: f32,
    /// Number of frame
    pub frame: u32,
}

impl Globals {
    pub fn reset(&mut self) {
        self.frame = 0;
        self.time = 0.0;
        self.mouse_wheel = 0.0;
    }
}

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vec2u {
    pub x: u32,
    pub y: u32,
}

impl Vec2u {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0, y: 0 }
    }
}

pub struct Vec3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3f {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

/// The globals we pass to the fragment shader
/// aligned to 32bit words
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Globals {
    /// Window resolution
    pub resolution: Vec2u,
    /// Mouse pos
    pub mouse: Vec2u,
    /// Mouse wheel
    pub mouse_wheel: f32,
    /// Draw area width/height ratio
    pub ratio: f32,
    /// Current running time in sec
    pub time: f32,
    /// Number of frame
    pub frame: u32,
}

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct UVec2 {
    pub x: u32,
    pub y: u32,
}

impl UVec2 {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0, y: 0 }
    }
}

/// The globals we pass to the fragment shader
/// aligned to 32bit words
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Globals {
    /// Window resolution
    pub resolution: UVec2,
    /// Mouse pos
    pub mouse: UVec2,
    /// Mouse wheel
    pub mouse_wheel: f32,
    /// Draw area width/height ratio
    pub ratio: f32,
    /// Current running time in sec
    pub time: f32,
    /// Number of frame
    pub frame: u32,
}

use std::path::PathBuf;

use crevice::std140;
use crevice::std430::AsStd430;
use mint::{Vector2, Vector3};

pub mod loader;
pub mod preprocessor;
pub mod renderer;

/// The globals we pass to the fragment shader
#[derive(AsStd430, Clone)]
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

pub enum Slider {
    Float {
        name: String,
        min: f32,
        max: f32,
        value: f32,
        default: f32,
    },
    Uint {
        name: String,
        value: u32,
        min: u32,
        max: u32,
        default: u32,
    },
    /*
    Int {
        name: String,
        value: i32,
        min: i32,
        max: i32,
        default: i32,
    },*/
    Bool {
        name: String,
        value: u32,
        default: u32,
    },
    Vec2 {
        name: String,
        value: Vector2<f32>,
        default: Vector2<f32>,
    },
    Vec3 {
        name: String,
        value: Vector3<f32>,
        default: Vector3<f32>,
    },
    Color {
        name: String,
        value: Vector3<f32>,
        default: Vector3<f32>,
    },
    /*
    Enum {
        name: String,
        value: u32,
        variants: Vec<String>,
        default: u32,
    },*/
}

macro_rules! reset_impl {
    ($enum:ident, $($item: ident )*) => (
        impl $enum {
            pub fn reset(&mut self) {
                match self {
                    $($enum::$item { value, default, .. } => {
                        *value = *default;
                    })*
                }
            }
        }
    )
}

reset_impl!(Slider, Float Uint Bool Vec2 Vec3 Color);

macro_rules! write_impl {
    ($align:ident, $enum:ident, $($item:ident )*) => {
        impl $enum {
            pub fn write<W: std::io::Write>(&self, writer: &mut crevice::$align::Writer<W>) {
                match self {
                    $($enum::$item { value, .. } => {
                        writer.write(value).unwrap();
                    })*
                }
            }
        }
    };
}

write_impl!(std140, Slider, Float Uint Bool Vec2 Vec3 Color);

/// Traverses the ast and extract useful data while converting the ast to valid glsl source
#[derive(Default)]
pub struct ShaderMetadata {
    pub sliders: Vec<Slider>,
    pub still_image: bool,
}

impl ShaderMetadata {
    pub fn params_buffer_size(&self) -> u64 {
        self.params_buffer().len() as u64
    }

    pub fn params_buffer(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut writer = std140::Writer::new(&mut bytes);

        for slider in self.sliders.iter() {
            slider.write(&mut writer);
        }

        bytes
    }

    pub fn reset_params(&mut self) {
        for slider in self.sliders.iter_mut() {
            slider.reset();
        }
    }
}

pub struct Shader {
    /// Path to the main shader file
    pub main: PathBuf,
    /// Path to all shader files that should be watched on
    pub sources: Vec<PathBuf>,
    /// Shader metadata extracted before compilation
    pub metadata: Option<ShaderMetadata>,
}

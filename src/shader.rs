use std::path::PathBuf;

use crevice::std140;
use mint::{Vector2, Vector3};

pub enum Slider {
    Float {
        name: String,
        min: f32,
        max: f32,
        value: f32,
        default: f32,
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
}

macro_rules! reset_impl {
    ($enum:ident, $($item: ident,)*) => (
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

reset_impl!(Slider, Float, Vec2, Vec3, Color,);

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

        for param in self.sliders.iter() {
            match param {
                Slider::Float { value, .. } => {
                    writer.write(value).unwrap();
                }
                Slider::Vec3 { value, .. } => {
                    writer.write(value).unwrap();
                }
                Slider::Color { value, .. } => {
                    writer.write(value).unwrap();
                }
                _ => {}
            }
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

use std::path::PathBuf;

use crate::types::Vec3f;

pub enum Slider {
    Float {
        name: String,
        min: f32,
        max: f32,
        value: f32,
    },
    Color {
        name: String,
        value: Vec3f,
    },
}

/// Traverses the ast and extract useful data while converting the ast to valid glsl source
pub struct ShaderMetadata {
    pub sliders: Vec<Slider>,
    pub still_image: bool,
}

impl Default for ShaderMetadata {
    fn default() -> Self {
        Self {
            sliders: Vec::new(),
            still_image: false,
        }
    }
}

impl ShaderMetadata {
    pub fn buffer_size(&self) -> u64 {
        let mut size = 0;
        for slider in self.sliders.iter() {
            match slider {
                &Slider::Float { .. } => {
                    size += 4;
                }
                &Slider::Color { .. } => {
                    size += 16;
                }
            }
        }
        size
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

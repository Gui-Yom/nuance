use std::path::PathBuf;

use crate::preprocessor::ShaderMetadata;

pub struct Shader {
    /// Path to the main shader file
    pub main: PathBuf,
    /// Path to all shader files that should be watched on
    pub sources: Vec<PathBuf>,
    /// Shader metadata extracted before compilation
    pub metadata: Option<ShaderMetadata>,
}

use std::borrow::Cow;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Result};
use log::{debug, error, warn};
use shaderc::{
    CompileOptions, Compiler, EnvVersion, GlslProfile, IncludeType, OptimizationLevel,
    ResolvedInclude, ShaderKind, SourceLanguage, TargetEnv,
};
use wgpu::ShaderSource;

use crate::shader::preprocessor;
use crate::shader::Shader;

pub struct ShaderLoader {
    compiler: Compiler,
    include_dirs: Vec<String>,
}

impl Default for ShaderLoader {
    fn default() -> Self {
        ShaderLoader {
            compiler: Compiler::new().expect("Can't create compiler"),
            include_dirs: Vec::with_capacity(4),
        }
    }
}

impl ShaderLoader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_include_dir(&mut self, include: &str) {
        self.include_dirs.push(include.to_string());
    }

    /// Load a shader, this will try to guess its type based on the file extension
    pub fn load_shader<P: AsRef<Path>>(&mut self, path: P) -> Result<(Shader, ShaderSource)> {
        let path = path.as_ref();
        // TODO collect all files necessary to compilation for watch
        match path.extension().and_then(|it| it.to_str()) {
            Some("spv") => {
                // sry for that terrible thing
                let data: Vec<u32> = fs::read(path)?.into_iter().map(|i| i as u32).collect();
                // We can't extract metadata from spirv modules
                Ok((
                    Shader {
                        main: path.to_path_buf(),
                        sources: vec![path.to_path_buf()],
                        metadata: None,
                    },
                    ShaderSource::SpirV(Cow::Owned(data)),
                ))
            }
            Some("glsl") | Some("frag") => {
                // Preprocess glsl to extract what we need
                let mut source = fs::read_to_string(path)?;
                let metadata = if let Ok((metadata, new)) = preprocessor::extract(&source) {
                    // We found params and transpiled the code
                    source = new;
                    Some(metadata)
                } else {
                    // No params extracted and source isn't modified
                    None
                };

                self.compile_shader(path.to_str().unwrap(), &source, "main")
                    .map(|it| {
                        (
                            Shader {
                                main: path.to_path_buf(),
                                sources: vec![path.to_path_buf()],
                                metadata,
                            },
                            it,
                        )
                    })
            }
            Some("wgsl") => Ok((
                // TODO extract data from wgsl
                Shader {
                    main: path.to_path_buf(),
                    sources: vec![path.to_path_buf()],
                    metadata: None,
                },
                ShaderSource::Wgsl(Cow::Owned(fs::read_to_string(path)?)),
            )),
            _ => Err(anyhow!("Unsupported shader format !")),
        }
    }

    /// Compile a shader from source to spirv in memory
    pub fn compile_shader(
        &mut self,
        name: &str,
        source: &str,
        entrypoint: &str,
    ) -> Result<ShaderSource<'_>> {
        let mut opts = CompileOptions::new().unwrap();
        opts.set_source_language(SourceLanguage::GLSL);
        opts.set_optimization_level(OptimizationLevel::Performance);
        opts.set_target_env(TargetEnv::Vulkan, EnvVersion::WebGPU as u32);
        //options.set_target_spirv(SpirvVersion::V1_5);
        opts.set_forced_version_profile(460, GlslProfile::None);

        let include_dirs = &self.include_dirs;
        opts.set_include_callback(move |name, include_type, source_file, _| {
            Self::find_include(include_dirs, name, include_type, source_file)
        });

        let compiled = self.compiler.compile_into_spirv(
            source,
            ShaderKind::Fragment,
            name,
            entrypoint,
            Some(&opts),
        )?;

        if compiled.get_num_warnings() > 0 {
            warn!(
                "Compilation warnings : \n{}",
                compiled.get_warning_messages()
            );
        }

        Ok(ShaderSource::SpirV(Cow::Owned(
            compiled.as_binary().to_owned(),
        )))
    }

    /// Resolve an include with the given name
    fn find_include(
        includes: &[String],
        name: &str,
        include_type: IncludeType,
        source_file: &str,
    ) -> Result<ResolvedInclude, String> {
        match include_type {
            IncludeType::Relative => {
                let local_inc = Path::new(source_file).parent().unwrap().join(name);
                // Search in the shader directory
                if local_inc.exists() {
                    Ok(ResolvedInclude {
                        resolved_name: local_inc.to_str().unwrap().to_string(),
                        content: fs::read_to_string(&local_inc).map_err(|e| e.to_string())?,
                    })
                } else {
                    // Search in registered include dirs
                    includes
                        .iter()
                        .find_map(|dir| {
                            let path = Path::new(dir).join(name);
                            if path.exists() {
                                Some(ResolvedInclude {
                                    resolved_name: path.to_str().unwrap().to_string(),
                                    content: fs::read_to_string(&path).ok().unwrap(),
                                })
                            } else {
                                None
                            }
                        })
                        .ok_or_else(|| "Include not found !".to_string())
                }
            }
            IncludeType::Standard => {
                // The nuance standard header
                if name == "Nuance" {
                    const STD: &str = include_str!("stdlib.glsl");
                    Ok(ResolvedInclude {
                        resolved_name: "NUANCE_STD".to_string(),
                        content: STD.to_string(),
                    })
                } else {
                    Err("No standard include with this name !".to_string())
                }
            }
        }
    }
}

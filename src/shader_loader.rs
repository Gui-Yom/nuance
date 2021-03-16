use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use log::debug;
use log::info;
use shaderc::{
    CompileOptions, Compiler, EnvVersion, GlslProfile, IncludeType, Limit, OptimizationLevel,
    ResolvedInclude, ShaderKind, SourceLanguage, TargetEnv,
};
use wgpu::ShaderSource;

pub struct ShaderLoader {
    compiler: Compiler,
    includes: Vec<String>,
}

impl ShaderLoader {
    pub fn new() -> Self {
        ShaderLoader {
            compiler: Compiler::new().expect("Can't create compiler"),
            includes: Vec::with_capacity(4),
        }
    }

    pub fn add_include(&mut self, include: &str) {
        self.includes.push(include.to_string());
    }

    /// Load a shader, this will try to guess its type based on the file extension
    pub fn load_shader<P: AsRef<Path>>(&mut self, path: P) -> Result<ShaderSource<'_>> {
        let path = path.as_ref();
        // Already compiled shader
        if path.extension().map_or(false, |e| e == "spv") {
            // sry for that terrible thing
            let data: Vec<u32> = fs::read(path)?.into_iter().map(|i| i as u32).collect();
            //debug!("data : {:#x?}", data);
            Ok(ShaderSource::SpirV(Cow::Owned(data)))
        } else if path
            .extension()
            .map_or(false, |e| e == "frag" || e == "glsl")
        {
            self.compile_shader(path.to_str().unwrap(), &fs::read_to_string(path)?, "main")
        } else {
            Err(anyhow!("Unsupported shader format !"))
        }
    }

    /// Compile a shader from source to spirv in memory
    pub fn compile_shader(
        &mut self,
        name: &str,
        source: &str,
        entrypoint: &str,
    ) -> Result<ShaderSource<'_>> {
        let includes = &self.includes;
        let mut options = CompileOptions::new().unwrap();
        options.set_source_language(SourceLanguage::GLSL);
        options.set_target_env(TargetEnv::Vulkan, EnvVersion::WebGPU as u32);
        options.set_optimization_level(OptimizationLevel::Performance);
        options.set_forced_version_profile(460, GlslProfile::None);
        options.set_include_callback(|name, inctype, source_file, source_line| match inctype {
            IncludeType::Relative => {
                let local_inc = Path::new(source_file).parent().unwrap().join(name);
                if local_inc.exists() {
                    Ok(ResolvedInclude {
                        resolved_name: local_inc.to_str().unwrap().to_string(),
                        content: fs::read_to_string(&local_inc).map_err(|e| e.to_string())?,
                    })
                } else {
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
                        .ok_or("Include not found !".to_string())
                }
            }
            IncludeType::Standard => {
                if name == "Shadertoy" {
                    const STD: &str = r#"
                    #define SHADERTOY
                    "#;
                    Ok(ResolvedInclude {
                        resolved_name: "SHADERTOY_BASE".to_string(),
                        content: STD.to_string(),
                    })
                } else {
                    Err("No standard include with this name !".to_string())
                }
            }
        });
        let compiled = self.compiler.compile_into_spirv(
            source,
            ShaderKind::Fragment,
            name,
            entrypoint,
            Some(&options),
        )?;
        Ok(ShaderSource::SpirV(Cow::Owned(
            compiled.as_binary().to_owned(),
        )))
    }
}

use std::borrow::{Borrow, Cow};
use std::fs;
use std::io::{BufReader, Read};
use std::path::Path;

use log::debug;
use shaderc::{CompileOptions, Compiler, EnvVersion, OptimizationLevel, ShaderKind, SourceLanguage, TargetEnv};
use wgpu::ShaderModuleSource;

pub struct ShaderLoader {
    compiler: Compiler
}

impl ShaderLoader {
    pub fn new() -> Self {
        ShaderLoader {
            compiler: Compiler::new().expect("Can't create compiler")
        }
    }

    /// Load a shader, this will try to guess its type based on the file extension
    pub fn load_shader<P: AsRef<Path>>(&mut self, path: P) -> Result<ShaderModuleSource<'_>, String> {
        let path = path.as_ref();
        if !path.exists() {
            return Err("File doesn't exist".to_string());
        }
        // Already compiled shader
        if path.extension().map_or(false, |e| e == "spv") {
            // sry for that terrible thing
            let data: Vec<u32> = unsafe { std::mem::transmute(fs::read(path).map_err(|e| format!("wtf {}", e))?) };
            //debug!("data : {:#x?}", data);
            Ok(ShaderModuleSource::SpirV(Cow::Owned(data)))
        } else if path.extension().map_or(false, |e| e == "frag") {
            self.compile_shader(path.to_str().unwrap(), &fs::read_to_string(path).unwrap(), "main")
        } else {
            Err("File isn't a GLSL nor a Spir-V fragment shader".to_string())
        }
    }

    /// Compile a shader from source to spirv in memory
    pub fn compile_shader(&mut self, name: &str, source: &str, entrypoint: &str) -> Result<ShaderModuleSource<'_>, String> {
        let mut options = CompileOptions::new().unwrap();
        // We specified we used GLSL so it should be good
        options.set_source_language(SourceLanguage::GLSL);
        // FIXME what if we don't run Vulkan ? or another version of Vulkan ?
        options.set_target_env(TargetEnv::Vulkan, EnvVersion::Vulkan1_2 as u32);
        options.set_optimization_level(OptimizationLevel::Performance);

        let compiled = self.compiler.compile_into_spirv(
            source,
            ShaderKind::Fragment,
            name,
            entrypoint,
            Some(&options),
        ).map_err(|e| format!("Compilation error : {}", e))?;
        Ok(ShaderModuleSource::SpirV(Cow::Owned(compiled.as_binary().to_owned())))
    }
}

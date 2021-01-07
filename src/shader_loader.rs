use std::borrow::Cow;
use std::fs;
use std::io::{BufReader, Read};

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

    /// Load a shader, this will try to guess its type based on the filename
    pub fn load_shader(&mut self, path: &str) -> Result<ShaderModuleSource<'_>, String> {

        // Already compiled shader
        if path.ends_with(".spv") {
            let data: Vec<u32> = unsafe { std::mem::transmute(fs::read(path).map_err(|e| format!("wtf {}", e))?) };
            //debug!("data : {:#x?}", data);
            Ok(ShaderModuleSource::SpirV(Cow::Owned(data)))
        } else if path.ends_with(".frag") {
            let mut options = CompileOptions::new().unwrap();
            // We specified we used GLSL so it should be good
            options.set_source_language(SourceLanguage::GLSL);
            // FIXME what if we don't run Vulkan ? or another version of Vulkan ?
            options.set_target_env(TargetEnv::Vulkan, EnvVersion::Vulkan1_2 as u32);
            options.set_optimization_level(OptimizationLevel::Performance);

            let compiled = self.compiler.compile_into_spirv(
                &fs::read_to_string(path).unwrap(),
                ShaderKind::Fragment,
                path,
                "main",
                Some(&options),
            ).unwrap();
            Ok(ShaderModuleSource::SpirV(Cow::Owned(compiled.as_binary().to_owned())))
        } else {
            Err("Can't read file format".to_string())
        }
    }
}

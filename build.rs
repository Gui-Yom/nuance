use std::fs;

use shaderc::{CompileOptions, Compiler, EnvVersion, OptimizationLevel, ShaderKind, SourceLanguage, TargetEnv};

fn main() {
    let files: Vec<_> = std::fs::read_dir("src/shaders").unwrap().map(|e| {
        let filename = format!("src/shaders/{}", e.unwrap().file_name().to_str().unwrap());
        if !filename.ends_with(".spv") {
            println!("cargo:rerun-if-changed={}", filename);
        }
        filename
    }).collect();

    let mut compiler = Compiler::new().unwrap();
    let mut options = CompileOptions::new().unwrap();
    options.set_source_language(SourceLanguage::GLSL);
    options.set_target_env(TargetEnv::Vulkan, EnvVersion::Vulkan1_2 as u32);
    options.set_optimization_level(OptimizationLevel::Performance);

    for file in files.iter() {
        let result = compiler.compile_into_spirv(
            &fs::read_to_string(file).unwrap(),
            if file.ends_with(".vert") { ShaderKind::Vertex } else { ShaderKind::Fragment },
            file,
            "main",
            Some(&options))
            .expect(&format!("Can't compile {}", file));
        fs::write(format!("{}.spv", file), result.as_binary_u8())
            .expect("Can't write compilation result to file");
    }
}
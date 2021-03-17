use anyhow::Result;
use log::info;
use shadyboi::shader_loader::ShaderLoader;
use spirv_reflect::ShaderModule;
use wgpu::ShaderSource;

fn main() -> Result<()> {
    let mut loader = ShaderLoader::new();
    let shader = loader.load_shader("shaders/purple.frag").unwrap();
    match shader {
        ShaderSource::SpirV(data) => {
            let module = ShaderModule::load_u32_data(data.as_ref()).unwrap();
            module
                .enumerate_output_variables(Some("main"))
                .unwrap()
                .iter()
                .for_each(|it| {
                    println!("{}, {:?}", it.name, it);
                });
            module
                .enumerate_descriptor_sets(Some("main"))
                .unwrap()
                .iter()
                .for_each(|it| {
                    it.bindings.iter().for_each(|it| {
                        println!("{}, {:?}", it.name, it);
                    });
                });
            module
                .enumerate_descriptor_bindings(Some("main"))
                .unwrap()
                .iter()
                .for_each(|it| {
                    println!("{}, {:?}", it.name, it);
                });
            module
                .enumerate_entry_points()
                .unwrap()
                .iter()
                .for_each(|it| {
                    println!("{}, {:?}", it.name, it);
                });
        }
        ShaderSource::Wgsl(_) => {}
    }

    Ok(())
}

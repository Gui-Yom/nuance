# Shadertoy

(Will be) a desktop equivalent to https://shadertoy.com.

Currently a good demo for wgpu-rs. Should be cross-platform.

### GPU Usage

By default it will use the first available low-power gpu that fills the criteria. Launch
with `shadertoy.exe high` to force the usage of a discrete gpu.

### Shaders

Shaders are written with the Vulkan flavor of GLSL (`#version 450`).
The [GL_KHR_vulkan_glsl](https://github.com/KhronosGroup/GLSL/blob/master/extensions/khr/GL_KHR_vulkan_glsl.txt)
extension is implicitly enabled. You can also use a shader already compiled to SpirV directly.

### TODO

- Error handling
- Hot GPU switch
- Mouse wheel input support
- Bind any custom value to uniforms
- Bind resources (like textures)
- Allow recording to gif or video
- GUI (imgui) to display informations (and interact ?)

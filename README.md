# Shadertoy

(Will be) a desktop equivalent to https://shadertoy.com.

Currently a good demo for wgpu-rs.

### Shaders

Shaders are written with the Vulkan flavor of GLSL (`#version 450`).
The [GL_KHR_vulkan_glsl](https://github.com/KhronosGroup/GLSL/blob/master/extensions/khr/GL_KHR_vulkan_glsl.txt)
extension is implicitly enabled. You can also use a SpirV shader directly.

### TODO

- Live reloading shaders (file watcher)
- Bind uniforms (through push constants)
    - any custom value ?
- Bind resources (like textures)
- Allow recording to gif or video
- Live CLI (write commands directly in the console)
- GUI (imgui) to display informations (and interact ?)

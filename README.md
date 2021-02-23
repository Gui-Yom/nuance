# Shadertoy

(Will be) a desktop equivalent to https://shadertoy.com.

Currently a good demo for wgpu-rs. Should be cross-platform.

## Usage

Run in your terminal with `shadertoy.exe`, this will open a preview window and display a terminal UI
for the logs. You can enter commands in your terminal to control the behavior of the simulation.

### Choose gpu

By default it will use the first available low-power gpu that fills the criteria. Launch
with `shadertoy.exe high` to force the usage of a discrete gpu.

### Commands

- `load <file>` to load a shader
- `reload` to reload the currently loaded shader
- `watch <file>` to watch for changes to `file` and reload the shader automatically
- `unwatch` to stop watching for changes
- `framerate <target_fps>` will limit the fps to `target_fps`
- `restart` to reset the globals
- `exit`

## Shaders

Shadertoy allows you tu run a custom fragment shader on the whole screen. Shaders are written with
the Vulkan flavor of GLSL (`#version 450`).
The [GL_KHR_vulkan_glsl](https://github.com/KhronosGroup/GLSL/blob/master/extensions/khr/GL_KHR_vulkan_glsl.txt)
extension is implicitly enabled. You can also use a shader already compiled to SpirV directly.

### Globals

Please include the following snippet in all your shaders to access those constants.

```glsl
layout(push_constant) uniform Globals {
// Window resolution
    uvec2 uResolution;
// Mouse position
    uvec2 uMouse;
// Aspect ratio
    float fRatio;
// Time in sec
    float uTime;
// The number of frame we're at
    uint uFrame;
};
```

## TODO

- Error handling
- Hot GPU switch
- Mouse wheel input support
- Support custom constants (push_constants preferred else storage buffers)
- Bind resources (like textures)
- Sound processing
- Allow saving to image, gif or video
- GUI (imgui or iced/druid) to display information (and interact ?)

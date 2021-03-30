# Nuance

(Will be) a desktop equivalent to https://shadertoy.com.

Currently a good demo for wgpu-rs. Should be cross-platform.

## Usage

Run in your terminal with `nuance.exe`, this will open a preview window and display a terminal UI
for the logs. You can enter commands in your terminal to control the behavior of the simulation.

### Choose gpu

By default it will use the first available low-power gpu that match the criteria. Launch
with `nuance.exe high` to force the usage of a discrete high-power gpu.

### Commands

- `load <file>` to load a shader
- `reload` to reload the currently loaded shader
- `watch <file>` to watch for changes to `file` and reload the shader automatically
- `unwatch` to stop watching for changes
- `framerate <target_fps>` will limit the fps to `target_fps`, default to 30
- `restart` to reset the globals
- `exit`

## Shaders

Nuance allows you tu run a custom fragment shader on the whole screen. Shaders are written with
the Vulkan flavor of GLSL (`#version 460`).
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
// Mouse wheel
    float fMouseWheel;
// Aspect ratio
    float fRatio;
// Time in sec
    float uTime;
// Incremented at each new frame
    uint uFrame;
};
```

### Custom parameters

You can specify additional parameters to your shader using an interface block.
When compiling your shader, parameters will be generated on the fly accordingly.
Sliders and other appropriate UI elements will appear on screen.
Example : 
```glsl
layout(params) uniform Params {
    layout(min = 0, max = 100, step = 1) float fSlider0;
    layout(min = 0, max = 20) float fSlider1;
};
```

### Examples

This repository includes some examples under `shaders/`. Some of these are not from me and are just
included here for demonstration purposes. They are the property of their respective owners.

## Development

We use `shaderc-rs` to compile shaders to spirv. It is therefore highly recommended to install the
vulkan sdk and set the `VULKAN_SDK` env var in order to find the prebuilt shaderc libraries. If not,
shaderc will download and build the vulkan libraries from source, which takes about 90% of this
entire application build time.

## TODO

- Merge params uniform block with push_constant block
- GUI with sliders (to interact with your shader values in realtime)
- Error handling (currently it crashes if something goes wrong)
- GPU hot switch (for when you see that you need some extra gpu juice)
- Bind textures as input
- Bind buffers as output
- Provide access to last rendered texture for stateful simulations
- Sound processing
- Save to image, gif or video

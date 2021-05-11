# Nuance

A nice tool to run your shaders on the gpu. Also a good demo for wgpu-rs.

## Installation

Install with cargo:

```shell
$ cargo install --locked nuance
```

Or download a prebuilt binary from the [Release](https://github.com/Gui-Yom/nuance/releases) page.
Prebuilt binaries are currently available for Windows (x86_64-pc-windows-msvc) and Linux
(x86_64-unknown-linux-gnu).

See [Development](#Development) when building from source.

## Usage

```shell
$ nuance shaders/color.frag
```

### Choose the gpu

By default it will use the first available low-power gpu that match the criteria. Launch
with `nuance.exe -H` to force the usage of a discrete high-power gpu.

## Shaders

Nuance allows you tu run a custom fragment shader on the whole screen. Shaders are written with the
Vulkan flavor of GLSL (`#version 460`) and an optional superset for generating sliders for your
shaders.
The [GL_KHR_vulkan_glsl](https://github.com/KhronosGroup/GLSL/blob/master/extensions/khr/GL_KHR_vulkan_glsl.txt)
extension is implicitly enabled. You can also use a shader already compiled to SpirV directly.

Please include the standard header `Nuance` for convenience.

```glsl
#include <Nuance>
```

### Globals

The standard header `Nuance` includes definitions for some useful globals :

```glsl
#define NUANCE

#version 460

layout(location = 0) out vec4 fragColor;

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

You can specify additional parameters to your shader using a special interface block. When compiling
your shader, parameters will be parsed from the source code. Sliders and other appropriate UI
elements will appear on screen. The shader source will then be transpiled to correct GLSL to be
compiled. Example :

```glsl
// layout(params) indicates that this block is the special one to be parsed.
layout(params) uniform Params {
// layout(min, max, init) to modify each parameters settings
    layout(min = 0, max = 100, init = 1) float fSlider0;
    layout(min = 0, max = 20) float fSlider1;
};

void main() {
    // You can use special values like <param>.min and <param>.max, they will be replaced by the settings defined
    // in the params block
    fragColor = vec4(fSlider0 / fSlider0.max, fSlider1 / fSlider1.max, 0.0, 1.0);
}
```

#### Why the layout qualifier ?

It's the only qualifier allowing any parameter inside, so we can comply with parser rules and make
your ide not throw red squiggy lines. We can change this later but this requires using a custom glgl
parser because qualifiers as usually built-ins.

### Examples

This repository includes some examples under `shaders/`. Some of these are not from me and are just
included here for demonstration purposes. They are the property of their respective owners.

## Development

We use `shaderc-rs` to compile shaders to spirv. It is therefore highly recommended to install the
vulkan sdk and set the `VULKAN_SDK` env var in order to find the prebuilt shaderc libraries. If not,
shaderc will download and build the vulkan libraries from source, which takes about 90% of this
entire application build time.

## TODO

- Reimplement commands with the GUI
- Merge params uniform block with push_constant block
- Error handling (currently it crashes if something goes wrong)
- GPU hot switch (for when you see that you need some extra gpu juice)
- Bind textures as input
- Bind buffers as output
- Provide access to last rendered texture for stateful simulations
- Sound processing
- Save to image, gif or video

# Nuance shaders

Shaders are preferably written with the Vulkan flavor of GLSL (`#version 460`).
The [GL_KHR_vulkan_glsl](https://github.com/KhronosGroup/GLSL/blob/master/extensions/khr/GL_KHR_vulkan_glsl.txt)
extension is implicitly enabled.

Some great resources to learn about shaders :

- [thebookofshaders.com](https://thebookofshaders.com/)

## Supported languages

Support         |GLSL|WGSL|Rust|SpirV
----------------|----|----|----|-----
Tier 0 / Import |✔️  |✔️  |    |✔️
Tier 1 / Std    |✔️  |    |    |
Tier 2 / Params |✔️  |    |    |

## Shader inputs

Access the current sample coordinates with `fragCoord`. The origin is the upper left. For normalized
0-1 coordinates, use `fragCoordNorm`.

You can also use a bunch of globals passed to your shader at each invocation to handle user input,
get canvas dimension and access time.

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
    float fTime;
// The number of frame we're at
    uint uFrame;
};
```

For used defined parameters, see [Parameters](#parameters).

## Shader output

```glsl
layout(location = 0) out vec4 fragColor;
```

Use `fragColor` to define the fragment color. Example :

```glsl
void main() {
    fragColor = vec4(1.0, 0.0, 0.0, 1.0);
}
```

## Conditional compilation

Compiling your shader with Nuance guarantees `NUANCE` is defined.

```glsl
#define NUANCE
```

You can use it for conditional compilation.

```glsl
#ifdef NUANCE
...
#else
...
#endif
```

## Special settings

You can set some settings in your shader with defines.

### Reactive or continuous rendering

Use `#define NUANCE_STILL_IMAGE` when your shader doesn't need continuous rendering because of an
animation. This prevents running it at a given framerate.
**This has no effect right now !**

## Parameters

Nuance allows you to define parameters for your shader. Before compiling your shader, parameters
will be parsed from the source code to generate sliders and other appropriate UI elements. Example :

```glsl
layout(params) uniform Params {
    layout(min = 0, max = 100, init = 1) float myValue;
    layout(min = 0, max = 20) float otherValue;
};
```

Each parameter UI appearance is derived from its type and qualifiers.

### Parameters types

type |qualifiers                |ui
-----|--------------------------|------------
float|min = ?, max = ?, init = ?|drag control
vec2 |init = ?                  |double drag control
vec3 |color, init = ?           |color picker
vec3 |init = ?                  |triple drag control
bool |init = ?                  |checkbox

### Special values

You can use the values you defined in the qualifiers using the dot notation. Those expressions will
be replaced at compile time. Example :

```glsl
void main() {
    fragColor = vec4(myValue / myValue.max, 0.0, 0.0, 1.0);
}
```

## Special values

### FIRST_RUN

```glsl
#define FIRST_RUN uFrame == 0
```

Has the value true if this shader invocation the first one since the last reset. Useful to setup an
initial state. **This define should not be used for conditional compilation !**

## Standard functions

By including the standard header `#include <Nuance>`, you also get access to some useful functions

### Noise

Utility functions around noise and randomness.

#### float noise(vec2)

Generates a pseudo random value from a vec2. The returned value only depends on the vec2 parameter.
Source : https://thebookofshaders.com/10/

### Previous render

Utility functions to sample the previous texture.

#### vec4 samplePrevious()

Sample the previously rendered texture at the current fragment coordinates.

#### vec4 samplePrevious(vec2)

Sample the previously rendered texture at the given coordinates.

#### vec4 samplePreviousN(vec2)

Sample the previously rendered texture at the given normalized coordinates.

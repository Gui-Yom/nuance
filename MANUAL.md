# Nuance shaders

Shaders are written with the Vulkan flavor of GLSL (`#version 460`).
The [GL_KHR_vulkan_glsl](https://github.com/KhronosGroup/GLSL/blob/master/extensions/khr/GL_KHR_vulkan_glsl.txt)
extension is implicitly enabled.

Some great resources to learn about shaders :

- [thebookofshaders.com](https://thebookofshaders.com/)

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
animation. This prevents running at a given framerate.

## Globals

Globals are special variables available to your shader.

```glsl
layout(push_constant) uniform Globals {
// Window resolution
    uvec2 uResolution;
// Mouse position
    uvec2 uMouse;
// Mouse wheel
    float iMouseWheel;
// Aspect ratio
    float fRatio;
// Time in sec
    float uTime;
// The number of frame we're at
    uint uFrame;
};
```

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
vec3 |color, init = ?           |color picker
vec3 |init = ?                  |triple drag control

### Special values

You can use the values you defined in the qualifiers using the dot notation. Those expressions will
be replaced at compile time. Example :

```glsl
void main() {
    fragColor = vec4(myValue / myValue.max, 0.0, 0.0, 1.0);
}
```

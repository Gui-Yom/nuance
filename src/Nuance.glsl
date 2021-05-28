#define NUANCE

#version 460

#define fragCoord gl_FragCoord

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
    float fTime;
// The number of frame we're at
    uint uFrame;
};

// Source : https://thebookofshaders.com/10/
float random(vec2 st) {
    return fract(sin(dot(st.xy, vec2(12.9898, 78.233))) * 43758.5453123);
}

#define NUANCE

#version 460

layout(location = 0) out vec4 fragColor;
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

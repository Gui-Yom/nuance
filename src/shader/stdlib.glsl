#define NUANCE

#version 460

#include <noise>

// Current fragment coordinates in pixel space
#define fragCoord gl_FragCoord
// Current fragment coordinates in normalized space
#define fragCoordNorm fragCoord.xy / uResolution

// Current fragment output color
layout(location = 0) out vec4 fragColor;

layout(set = 0, binding = 0) uniform texture2D lastFrame;
layout(set = 0, binding = 1) uniform sampler lastFrameSampler;

// Globals are variables your shader can access
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

#define FIRST_RUN uFrame == 0

// Sample the last frame at the given normalized coordinates
vec4 samplePreviousN(vec2 st) {
    return texture(sampler2D(lastFrame, lastFrameSampler), st);
}

// Sample last frame at the given coordinates in pixel coordinates
vec4 samplePrevious(vec2 xy) {
    return samplePreviousN(xy / uResolution);
}

// Sample last frame at the current fragment coordinates
vec4 samplePrevious() {
    return samplePreviousN(fragCoordNorm);
}

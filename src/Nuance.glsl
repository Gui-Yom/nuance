#define NUANCE

#version 460

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

// Generate a pseudo random value from a vec2
// Source : https://thebookofshaders.com/10/
float noise(vec2 st) {
    return fract(sin(dot(st, vec2(12.9898, 78.233))) * 43758.5453123);
}

// Sample the last frame at the given normalized coordinates
vec4 sampleLastFrameNorm(vec2 st) {
    return texture(sampler2D(lastFrame, lastFrameSampler), st);
}

// Sample last frame at the given coordinates in pixel coordinates
vec4 sampleLastFrame(vec2 xy) {
    return sampleLastFrameNorm(xy / uResolution);
}

// Sample last frame at the current fragment coordinates
vec4 sampleLastFrame() {
    return sampleLastFrameNorm(fragCoordNorm);
}

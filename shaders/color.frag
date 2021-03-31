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

void main() {
    fragColor = vec4(0.0, (sin(2. * uTime - gl_FragCoord.x / uResolution.x) + 1.) / 2., (cos(2. * uTime - gl_FragCoord.y / uResolution.y) + 1.) / 2., 1.0);
}

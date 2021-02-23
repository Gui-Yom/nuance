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
    uvec2 center = uResolution / 2;
    float r = length(gl_FragCoord.xy - center);
    if (r <= 20) {
        fragColor = vec4(1.0, 0.0, 0.0, 1.0);
    } else if (r <= (sin(uFrame / 8.0) + 1.0) / 2.0 * 200 + 40) {
        fragColor = vec4(0.0, (sin(uTime * 2.0) + 1.0) / 2.0, 0.0, 1.0);
    } else {
        fragColor = vec4(0.0, 0.0, 1.0, 1.0);
    }
}

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
layout(params) uniform Params {
    layout(min = 0.0, max = 100.0, init = 50.0) float fSlider0;
    layout(min = 0, max = 20.0) float fSlider1;
    layout(min = 0, max = 1) float fSlider2;
};

void main() {
    fragColor = vec4(fSlider0, fSlider1, fSlider2, 1.0);
    //fragColor = vec4(1.0, 0.0, 1.0, 1.0);
}

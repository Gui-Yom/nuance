#version 460

layout(location = 0) out vec4 outColor;
layout(push_constant) uniform Globals {
// Time in ms
    uint uTime;
// Width in pixels
    uint uWidth;
// Height in pixels
    uint uHeight;
// Aspect ratio
    float fRatio;
};

void main() {
    vec2 center = vec2(uWidth / 2, uHeight / 2);
    float r = length(gl_FragCoord.xy - center);
    if (r <= 10) {
        outColor = vec4(1.0, 0.0, 0.0, 1.0);
    } else if (r <= uTime / 60 + 10) {
        outColor = vec4(0.0, 1.0, 0.0, 1.0);
    } else {
        outColor = vec4(0.0, 0.0, 1.0, 1.0);
    }
}

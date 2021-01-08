#version 460

layout(location = 0) out vec4 outColor;
layout(binding = 0) uniform Globals {
    uint Time;
};

void main() {
    vec2 center = vec2(400 * 1.25, 300 * 1.25);
    float r = length(gl_FragCoord.xy - center);
    if (r <= 10) {
        outColor = vec4(1.0, 0.0, 0.0, 1.0);
    } else if (r <= Time / 80 + 10) {
        outColor = vec4(0.0, 1.0, 0.0, 1.0);
    } else {
        outColor = vec4(0.0, 0.0, 1.0, 1.0);
    }
}

#version 460

layout(location = 0) out vec4 outColor;
layout(binding = 0) uniform Globals {
    float value;
};

void main() {
    vec2 center = vec2(400 * 1.25, 300 * 1.25);
    float r = length(gl_FragCoord.xy - center);
    if (r <= 100) {
        outColor = vec4(1.0, 0.0, 0.0, 1.0);
    } else if (r <= value) {
        outColor = vec4(0.0, 1.0, 0.0, 1.0);
    } else {
        outColor = vec4(0.0, 0.0, 1.0, 1.0);
    }
}

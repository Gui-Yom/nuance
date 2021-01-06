#version 460

layout(location = 0) out vec4 outColor;

void main() {
    vec2 center = vec2(400 * 1.25, 300 * 1.25);
    float r = length(gl_FragCoord.xy - center);
    outColor = vec4((sin(1.0 / 20 * r) + 1.0) / 2.0, (cos(1.0 / 20 * r) + 1.0) / 2.0, (tan(1.0 / 20 * r) + 1.0) / 2.0, 1.0);
}

#version 460

layout(location = 0) out vec4 fragColor;

void main() {
    vec2 center = vec2(400 * 1.25, 300 * 1.25);
    float r = length(gl_FragCoord.xy - center);
    float omega = 1.0 / 20 * r;
    fragColor = vec4((sin(omega) + 1.0) / 2.0, (cos(omega) + 1.0) / 2.0, (tan(omega) + 1.0) / 2.0, 1.0);
}

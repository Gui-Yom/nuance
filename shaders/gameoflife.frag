#include <Nuance>

void main() {
    if (FIRST_RUN) {
        float value = noise(fragCoordNorm);
        fragColor = vec4(float(value > 0.5), 0.0, 0.0, 1.0);
    } else {
        float topL = sampleLastFrame(fragCoord.xy + vec2(-1, -1)).x;
        float top = sampleLastFrame(fragCoord.xy + vec2(0, -1)).x;
        float topR = sampleLastFrame(fragCoord.xy + vec2(1, -1)).x;
        float L = sampleLastFrame(fragCoord.xy + vec2(-1, 0)).x;
        uint value = uint(sampleLastFrame().x == 1.0);
        float R = sampleLastFrame(fragCoord.xy + vec2(1, 0)).x;
        float botL = sampleLastFrame(fragCoord.xy + vec2(-1, 1)).x;
        float bot = sampleLastFrame(fragCoord.xy + vec2(0, 1)).x;
        float botR = sampleLastFrame(fragCoord.xy + vec2(1, 1)).x;

        uint count = uint(topL + top + topR + L + R + botL + bot + botR);

        value = uint(value == 0 ? count == 3 : (count == 2 || count == 3));

        fragColor = vec4(float(value), 0.0, 0.0, 1.0);
    }
}

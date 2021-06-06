#include <Nuance>

void main() {
    if (FIRST_RUN) {
        float value = noise(fragCoordNorm);
        fragColor = vec4(float(value > 0.5), 0.0, 0.0, 1.0);
    } else {
        float topL = samplePrevious(fragCoord.xy + vec2(-1, -1)).x;
        float top = samplePrevious(fragCoord.xy + vec2(0, -1)).x;
        float topR = samplePrevious(fragCoord.xy + vec2(1, -1)).x;
        float L = samplePrevious(fragCoord.xy + vec2(-1, 0)).x;
        uint value = uint(samplePrevious().x == 1.0);
        float R = samplePrevious(fragCoord.xy + vec2(1, 0)).x;
        float botL = samplePrevious(fragCoord.xy + vec2(-1, 1)).x;
        float bot = samplePrevious(fragCoord.xy + vec2(0, 1)).x;
        float botR = samplePrevious(fragCoord.xy + vec2(1, 1)).x;

        uint count = uint(topL + top + topR + L + R + botL + bot + botR);

        value = uint(value == 0 ? count == 3 : (count == 2 || count == 3));

        fragColor = vec4(float(value), 0.0, 0.0, 1.0);
    }
}

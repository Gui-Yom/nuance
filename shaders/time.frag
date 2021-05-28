#include <Nuance>

void main() {
    uvec2 center = uResolution / 2;
    float r = length(fragCoord.xy - center);
    if (r <= 20) {
        fragColor = vec4(1.0, 0.0, 0.0, 1.0);
    } else if (r <= (sin(uFrame / 8.0) + 1.0) / 2.0 * 200 + 40) {
        fragColor = vec4(0.0, (sin(fTime * 2.0) + 1.0) / 2.0, 0.0, 1.0);
    } else {
        fragColor = vec4(0.0, 0.0, 1.0, 1.0);
    }
}

#include <Nuance>

void main() {
    float r = length(fragCoord.xy - uMouse);
    if (r <= 60 * (fMouseWheel + 1.0)) {
        fragColor = vec4(1.0, 0.0, 0.0, 1.0);
    } else {
        fragColor = vec4(0.0, 0.0, 0.0, 1.0);
    }
}

#include <Nuance>

void main() {
    float r = length(gl_FragCoord.xy - uMouse);
    if (r <= 60 * (iMouseWheel + 1.0)) {
        fragColor = vec4(1.0, 0.0, 0.0, 1.0);
    } else {
        fragColor = vec4(0.0, 0.0, 0.0, 1.0);
    }
}

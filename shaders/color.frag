#include <Nuance>

void main() {
    fragColor = vec4(0.0, (sin(2. * uTime - gl_FragCoord.x / uResolution.x) + 1.) / 2., (cos(2. * uTime - gl_FragCoord.y / uResolution.y) + 1.) / 2., 1.0);
}

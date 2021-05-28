#include <Nuance>

void main() {
    fragColor = vec4(0.0, (sin(2. * fTime - fragCoord.x / uResolution.x) + 1.) / 2., (cos(2. * fTime - fragCoord.y / uResolution.y) + 1.) / 2., 1.0);
}

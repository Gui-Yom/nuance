#include <Nuance>

#define WAVELENGTH 100

layout(params) uniform Params {
    layout(max = 2) uint noiseType;
    bool lsd;
};

void main() {

    if (!lsd) {
        float value;
        switch (noiseType) {
            case 0:
            value = noise(fragCoord.xy);
            break;
            case 1:
            value = noiseB(fragCoord.xy);
            break;
            case 2:
            value = noiseVoronoi(fragCoord.xy, WAVELENGTH, vec2(0, 0));
            break;
        }
        fragColor = vec4(value, value, value, 1.0);
    } else {
        vec3 value;
        switch (noiseType) {
            case 0:
            value = vec3(noise(fragCoord.xy), noise(fragCoord.xy), noise(fragCoord.x * fragCoord.y));
            break;
            case 1:
            value = vec3(noiseB(fragCoord.xy), noiseB(fragCoord.xy), noiseB(fragCoord.x * fragCoord.y));
            break;
            case 2:
            value = vec3(noiseVoronoi(fragCoord.xy, WAVELENGTH, vec2(0, 0)), noiseVoronoi(fragCoord.xy, WAVELENGTH, vec2(0, 0)), noiseVoronoi(fragCoord.xy, WAVELENGTH, vec2(0, 0)));
            break;
        }
        fragColor = vec4(value, 1.0);
    }
}
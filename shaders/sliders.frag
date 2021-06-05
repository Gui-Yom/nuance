#include <Nuance>

#define NUANCE_STILL_IMAGE

layout(params) uniform Params {
    layout(color, init = vec3(0.0, 0.0, 1)) vec3 rgb;
    layout(min = 0, max = 1) float a;
    vec3 pos;
    vec2 b;
    bool c;
};

void main() {
    float r = length(fragCoord.xy - pos.xy);
    if (r <= 60 * (fMouseWheel + 1.0)) {
        fragColor = vec4(c ? 1.0 : 0.0, a, 0.0, 1.0);
    } else {
        fragColor = vec4(rgb, 1.0);
    }
}

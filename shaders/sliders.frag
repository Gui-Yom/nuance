#include <Nuance>

#define NUANCE_STILL_IMAGE

//layout(params) uniform Params {
//    layout(min = 0.0, max = 1.0) float red;
//    layout(min = 0.0, max = 1.0) float green;
//    layout(min = 0.0, max = 1.0, init = 0.5) float blue;
//};
//
//void main() {
//    fragColor = vec4(red, green, blue, 1.0);
//}

layout(params) uniform Params {
    layout(color, init = vec3(0.0, 0.0, 1)) vec3 rgb;
    layout(min = 0, max = 1) float a;
    vec3 pos;
    vec2 b;
};

void main() {
    float r = length(gl_FragCoord.xy - pos.xy);
    if (r <= 60 * (iMouseWheel + 1.0)) {
        fragColor = vec4(1.0, a, 0.0, 1.0);
    } else {
        fragColor = vec4(rgb, 1.0);
    }
}

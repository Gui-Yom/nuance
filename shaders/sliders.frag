#include <Nuance>

#define NUANCE_STILL_IMAGE

layout(params) uniform Params {
    layout(min = 0.0, max = 1.0) float red;
    layout(min = 0.0, max = 1.0) float green;
    layout(min = 0.0, max = 1.0, init = 0.5) float blue;
};

void main() {
    fragColor = vec4(red, green, blue, 1.0);
}

//layout(params) uniform Params {
//    layout(color) vec3 rgb;
//};
//
//void main() {
//    fragColor = vec4(rgb, 1.0);
//}

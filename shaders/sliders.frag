#include <Nuance>

layout(params) uniform Params {
    layout(min = 0.0, max = 1.0) float red;
    layout(min = 0.0, max = 1.0) float green;
    layout(min = 0.0, max = 1.0, init = 0.5) float blue;
};

void main() {
    fragColor = vec4(red, green, blue, 1.0);
}

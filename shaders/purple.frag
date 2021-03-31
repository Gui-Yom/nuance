#include <Nuance>

layout(params) uniform Params {
    layout(min = 0.0, max = 100.0, init = 50.0) float fSlider0;
    layout(min = 0, max = 20.0) float fSlider1;
    layout(min = 0, max = 1) float fSlider2;
};

void main() {
    fragColor = vec4(fSlider0, fSlider1, fSlider2, 1.0);
    //fragColor = vec4(1.0, 0.0, 1.0, 1.0);
}

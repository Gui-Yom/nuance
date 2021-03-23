#version 460

layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 0) uniform Block {
    float fSlider0;
    float fSlider1;
    float fSlider2;
};

void main() {
    fragColor = vec4(fSlider0, fSlider1, fSlider2, 1.0);
}

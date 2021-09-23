#include <Nuance>

layout(params) uniform Params {
    bool showColors;
    layout(init = 0.98) float fade;
	layout(init = 0.999) float quantity;
};

float sampleAroundAvg() {
    float a = samplePrevious(fragCoord.xy + vec2(-1.0, -1.0)).x;
    float b = samplePrevious(fragCoord.xy + vec2(-1.0, 0.0)).x;
    float c = samplePrevious(fragCoord.xy + vec2(-1.0, 1.0)).x;
    float d = samplePrevious(fragCoord.xy + vec2(0.0, -1.0)).x;
    float e = samplePrevious(fragCoord.xy + vec2(0.0, 1.0)).x;
    float f = samplePrevious(fragCoord.xy + vec2(1.0, -1.0)).x;
    float g = samplePrevious(fragCoord.xy + vec2(1.0, 0.0)).x;
    float h = samplePrevious(fragCoord.xy + vec2(1.0, 1.0)).x;
    return (a + b + c + d + e + f + g + h) / 8.0;
}

float sampleAroundMax() {
    float a = samplePrevious(fragCoord.xy + vec2(-1.0, -1.0)).x;
    float b = samplePrevious(fragCoord.xy + vec2(-1.0, 0.0)).x;
    float c = samplePrevious(fragCoord.xy + vec2(-1.0, 1.0)).x;
    float d = samplePrevious(fragCoord.xy + vec2(0.0, -1.0)).x;
    float e = samplePrevious(fragCoord.xy + vec2(0.0, 1.0)).x;
    float f = samplePrevious(fragCoord.xy + vec2(1.0, -1.0)).x;
    float g = samplePrevious(fragCoord.xy + vec2(1.0, 0.0)).x;
    float h = samplePrevious(fragCoord.xy + vec2(1.0, 1.0)).x;
    return max(a, max(b, max(c, max(d, max(e, max(f, max(g, h)))))));
}

void main() {
    float r = length(fragCoord.xy - uMouse);

    float noise = noiseVoronoi(fragCoord.xy, fTime, vec2(noise(fragCoord.xy), noise(vec2(fTime / 10, fTime * 1.5))));

    float intensity = 0.0;
    if (noise >= quantity) {
        intensity = 1.0;
    } else {
        intensity = max(0, sampleAroundMax() * fade);
    }

    fragColor = vec4(intensity, intensity * 0.4, 0.0, 1.0);
}

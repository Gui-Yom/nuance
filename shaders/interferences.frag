#version 460

layout(location = 0) out vec4 fragColor;
layout(push_constant) uniform Globals {
// Window resolution
    uvec2 uResolution;
// Mouse position
    uvec2 uMouse;
// Mouse wheel
    float iMouseWheel;
// Aspect ratio
    float fRatio;
// Time in sec
    float uTime;
// The number of frame we're at
    uint uFrame;
};

#define PI 3.1415926538
#define BIAS 16.0
#define D 50
#define d 2
#define V 340
#define f 1000

#define TOPVIEW

float remap(float minval, float maxval, float curval) {
    return ( curval - minval ) / ( maxval - minval );
}

const vec3 s1 = vec3(d/2, 0, 0);
const vec3 s2 = vec3(-d/2, 0, 0);

void main() {
	float w = 2.0 * PI * f;
	vec3 point = vec3((gl_FragCoord.xy / uResolution - 0.5) * 32, D);
    float value = cos(w * uTime * 0.0005 - w / V * distance(point, s1));
	float value2 = cos(w * uTime * 0.0005 - w / V * distance(point, s2));
	
    #ifdef TOPVIEW

    fragColor = vec4(remap(0.0, 2.0, abs(value + value2)), 0.0, 0.0, 1.0);

    #else

    float dist = abs(gl_FragCoord.y - value * uResolution.y);
    fragColor = vec4(dist <= BIAS, 0.0, 0.0, 1.0);

    #endif
}

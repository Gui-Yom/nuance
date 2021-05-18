// Ether by nimitz 2014 (twitter: @stormoid)
// https://www.shadertoy.com/view/MsjSW3
// License Creative Commons Attribution-NonCommercial-ShareAlike 3.0 Unported License
// Contact the author for other licensing options

#include <Nuance>

mat2 m(float a) {
    float c = cos(a), s = sin(a);
    return mat2(c, -s, s, c);
}

float map(vec3 p) {
    p.xz *= m(uTime * 0.4);
    p.xy *= m(uTime * 0.3);
    vec3 q = p * 2.0 + uTime;
    return length(p + vec3(sin(uTime * 0.7))) * log(length(p) + 1.0) + sin(q.x + sin(q.z + sin(q.y))) * 0.5 - 1.0;
}

void main() {
    vec3 p = vec3(gl_FragCoord.xy / uResolution.y - vec2(.5, .5), -1.0);
    vec3 cl = vec3(0.);
    float d = 2.5;
    for (int i=0; i<=5; i++) {
        vec3 p = vec3(0, 0, 5.) + normalize(p) * d;
        float rz = map(p);
        float f =  clamp((rz - map(p + .1)) * 0.5, -.1, 1.);
        vec3 l = vec3(0.1, 0.3, .4) + vec3(5., 2.5, 3.) * f;
        cl = cl * l + smoothstep(2.5, .0, rz) * .7 * l;
        d += min(rz, 1.);
    }
    fragColor = vec4(cl, 1.0);
}

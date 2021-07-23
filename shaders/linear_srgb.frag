#include <Nuance>

vec3 to_linear(vec3 srgb) {
    bvec3 cutoff = lessThan(srgb, vec3(0.04045));
    vec3 lower = srgb / vec3(12.92);
    vec3 higher = pow((srgb + vec3(0.055)) / vec3(1.055), vec3(2.4));
    return mix(higher, lower, cutoff);
}

vec3 to_srgb(vec3 linear) {
    bvec3 cutoff = lessThan(linear, vec3(0.0031308));
    vec3 lower = linear * vec3(12.92);
    vec3 higher = vec3(1.055) * pow(linear, vec3(1 / 2.4)) - vec3(0.055);
    return mix(higher, lower, cutoff);
}

void main() {
    // Linear interpolation between green and red
    vec3 a = vec3(0.0, 1.0, 0.0);
    vec3 b = vec3(1.0, 0.0, 0.0);
    if (fragCoord.y < uResolution.y / 3) {
        // No correction (the correct one)
        fragColor = vec4(mix(a, b, fragCoord.x / uResolution.x), 1.0);
    } else if (fragCoord.y < uResolution.y * 2. / 3) {
        // To srgb
        fragColor = vec4(to_srgb(mix(a, b, fragCoord.x / uResolution.x)), 1.0);
    } else {
        // To linear
        fragColor = vec4(to_linear(mix(a, b, fragCoord.x / uResolution.x)), 1.0);
    }
}
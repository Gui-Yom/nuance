#include <Nuance>

#define MAX_ITER 1000

void main() {
    vec2 pos = mix(vec2(-2.5, -1), vec2(1.0, 1.0), gl_FragCoord.xy / uResolution);
    vec2 c = vec2(0);
    uint iter = 0;
    while (dot(c, c) <= 4 && iter < MAX_ITER) {
        float temp = c.x * c.x - c.y * c.y + pos.x;
        c.y = 2 * c.x * c.y + pos.y;
        c.x = temp;
        iter += 1;
    }
    float color = 1 - iter / MAX_ITER;
    fragColor = vec4(color, color, color, 1.0);
}

// Generate a pseudo random value from a vec2
// Source : https://thebookofshaders.com/10/
float noise(float u) {
    return fract(sin(u) * 43758.5453123);
}

float noise(vec2 uv) {
    return fract(sin(dot(uv, vec2(12.9898, 78.233))) * 43758.5453123);
}

// Generate a pseudo random value from a vec3
float noise(vec3 uvw) {
    return fract(sin(dot(uvw, vec3(12.9898, 78.233, 144.7272))) * 43758.5453);
}

float noiseB(float u) {
    float fl = floor(u);
    float fc = fract(u);
    return mix(noise(fl), noise(fl + 1.0), fc);
}

float noiseB(vec2 uv) {
    const vec2 d = vec2(0.0, 1.0);
    vec2 b = floor(uv), f = smoothstep(vec2(0.0), vec2(1.0), fract(uv));
    return mix(mix(noise(b), noise(b + d.yx), f.x), mix(noise(b + d.xy), noise(b + d.yy), f.x), f.y);
}

float noiseVoronoi(in float x, in float y, in float xrand, in float yrand) {
    float integer_x = x - fract(x);
    float fractional_x = x - integer_x;

    float integer_y = y - fract(y);
    float fractional_y = y - integer_y;

    float val[4];

    val[0] = noise(vec2(integer_x, integer_y));
    val[1] = noise(vec2(integer_x+1.0, integer_y));
    val[2] = noise(vec2(integer_x, integer_y+1.0));
    val[3] = noise(vec2(integer_x+1.0, integer_y+1.0));

    float xshift[4];

    xshift[0] = xrand * (noise(vec2(integer_x+0.5, integer_y)) - 0.5);
    xshift[1] = xrand * (noise(vec2(integer_x+1.5, integer_y)) -0.5);
    xshift[2] = xrand * (noise(vec2(integer_x+0.5, integer_y+1.0))-0.5);
    xshift[3] = xrand * (noise(vec2(integer_x+1.5, integer_y+1.0))-0.5);

    float yshift[4];

    yshift[0] = yrand * (noise(vec2(integer_x, integer_y +0.5)) - 0.5);
    yshift[1] = yrand * (noise(vec2(integer_x+1.0, integer_y+0.5)) -0.5);
    yshift[2] = yrand * (noise(vec2(integer_x, integer_y+1.5))-0.5);
    yshift[3] = yrand * (noise(vec2(integer_x+1.5, integer_y+1.5))-0.5);

    float dist[4];

    dist[0] = sqrt((fractional_x + xshift[0]) * (fractional_x + xshift[0]) + (fractional_y + yshift[0]) * (fractional_y + yshift[0]));
    dist[1] = sqrt((1.0 -fractional_x + xshift[1]) * (1.0-fractional_x+xshift[1]) + (fractional_y +yshift[1]) * (fractional_y+yshift[1]));
    dist[2] = sqrt((fractional_x + xshift[2]) * (fractional_x + xshift[2]) + (1.0-fractional_y +yshift[2]) * (1.0-fractional_y + yshift[2]));
    dist[3] = sqrt((1.0-fractional_x + xshift[3]) * (1.0-fractional_x + xshift[3]) + (1.0-fractional_y +yshift[3]) * (1.0-fractional_y + yshift[3]));

    int i, i_min;
    float dist_min = 100.0;
    for (i=0; i<4;i++) {
        if (dist[i] < dist_min)
        {
            dist_min = dist[i];
            i_min = i;
        }
    }

    return val[i_min];
}

float noiseVoronoi(in vec2 coord, in float wavelength, in vec2 rand) {
    return noiseVoronoi(coord.x / wavelength, coord.y / wavelength, rand.x, rand.y);
}

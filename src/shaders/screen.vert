#version 460

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
    vec2 position = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
    gl_Position = vec4(position * 2.0f + -1.0f, 0.0f, 1.0f);
}

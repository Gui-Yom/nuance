[[stage(vertex)]]
fn main([[builtin(vertex_index)]] in_vertex_index: u32) -> [[builtin(position)]] vec4<f32> {
    let pos = vec2<f32>(f32((in_vertex_index << 1u) & 2u), f32(in_vertex_index & 2u));
    return vec4<f32>(pos * 2.0 - 1.0, 0.0, 1.0);
}

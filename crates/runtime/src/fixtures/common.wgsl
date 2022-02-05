fn get_coords(in_vertex_index: u32) -> vec2<f32> {
    let x = f32(i32(in_vertex_index) - 1);
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1);
    return vec2<f32>(x, y);
}
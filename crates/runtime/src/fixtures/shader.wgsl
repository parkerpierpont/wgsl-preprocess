#import test:common

@stage(vertex)
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(in_vertex_index) - 1);
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

@stage(fragment)
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}

///
/// 0
/// x = 0 - 1 = -1
/// y = 0000
///     0001
///     (0 * 2) - 1
///     = -1
/// 1
/// x = 1 - 1 = 0
/// y = 0001
///     0001
///     (1 * 2) - 1
///     = 1
/// 2
/// x = 2 - 1 = 1
/// y = 0001
///     0010
///     (0 * 2) - 1
///     = -1
///
/// 0 = (-1, -1)
/// 1 = ( 0, 1 )
/// 2 = ( 1,-1 )
///
///               1
///    ___________y__________
///    |          |         |
///    |          |         |
///  -1|__________0_________|1
///    |          |         |
///    |          |         |
///    x__________|_________z
///             -1
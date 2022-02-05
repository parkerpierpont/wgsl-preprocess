use skia_safe::surface::Surface as SkiaSurface;
mod cpu;
mod d3d;
mod gl;
mod metal;
mod vulkan;

pub trait AnilineDriver {
    fn new_surface() -> SkiaSurface;
}

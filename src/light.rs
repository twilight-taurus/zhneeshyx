use cgmath::*;
use winit::event::*;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformBuffer {
    position: [f32; 3], // 12 bytes
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    _padding_position: u32, // 4 bytes

    color: [f32; 3], // 12 bytes
    _padding_color: u32, // 4 bytes
    // ...
}

impl UniformBuffer {
    pub fn new() -> Self {
        Self {
            position: [2.0, 2.0, 2.0],
            _padding_position: 0,
            color: [1.0, 1.0, 1.0],
            _padding_color: 0,
        }
    }
}
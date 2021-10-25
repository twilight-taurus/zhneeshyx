use image::GenericImageView;
use anyhow::*;

// Textures: Efficient way of rendering highly detailed objects.
// -> images overlayed on a triangle mesh.

/*
    There are multiple types of textures 
    such as normal maps, 
    bump maps, 
    specular maps and 
    diffuse maps (or simply: the color texutre)
*/

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}
use cgmath::*;
use winit::event::*;
use winit::dpi::PhysicalPosition;
use std::time::Duration;
use std::f32::consts::FRAC_PI_2;

use std::cell::Cell;

#[derive(Debug)]
pub struct CameraController {

    // use cell for the bools.
    // -> that way the event handlers only take mutable references
    // if they have to.
    move_left: Cell<bool>,
    move_right: Cell<bool>,
    move_forward: Cell<bool>,
    move_backward: Cell<bool>,
    move_up: Cell<bool>,
    move_down: Cell<bool>,

    move_speed: f32,

    rotate: Cell<bool>,
    rotate_horizontal : Cell<f32>,
    rotate_vertical: Cell<f32>,
    
    rotate_speed: f32,

    scroll_speed: f32,
    sensitivity: f32,
}


impl CameraController {
    pub fn new() -> Self {
        Self {
            move_left: Cell::new(false),
            move_right: Cell::new(false),
            move_forward: Cell::new(false),
            move_backward: Cell::new(false),
            move_up: Cell::new(false),
            move_down: Cell::new(false),

            move_speed: 1.0,

            rotate: Cell::new(false),
            rotate_horizontal : Cell::new(0.0),
            rotate_vertical: Cell::new(0.0),

            rotate_speed: 1.0,

            scroll_speed: 1.0,
            sensitivity: 1.0,
        }
    }

    pub fn process_keydown(&self, key: VirtualKeyCode) {
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.move_forward.set(true);
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.move_backward.set(true);
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.move_left.set(true);
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.move_right.set(true);
            }
            VirtualKeyCode::Space => {
                self.move_up.set(true);
            }
            VirtualKeyCode::LShift => {
                self.move_down.set(true);
            }
            _ => (),
        }
    }

    pub fn process_keyup(&self, key: VirtualKeyCode) {
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.move_forward.set(false);
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.move_backward.set(false);
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.move_left.set(false);
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.move_right.set(false);
            }
            VirtualKeyCode::Space => {
                self.move_up.set(false);
            }
            VirtualKeyCode::LShift => {
                self.move_down.set(false);
            }
            _ => (),
        }
    }
/*
    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            // I'm assuming a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition {
                y: scroll,
                ..
            }) => *scroll as f32,
        };
    }
*/
    pub fn update_camera(&mut self, camera: &mut Camera) {
        use cgmath::InnerSpace;

        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

       // Prevents glitching when camera gets too close to the
        // center of the scene.
        if self.move_forward.get() /*&& ( forward_mag > self.move_speed ) */ {
            camera.eye += forward_norm * self.move_speed;
        }
        if self.move_backward.get() {
            camera.eye -= forward_norm * self.move_speed;
        }

        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case the up/ down is pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.magnitude();

        if self.move_right.get() {
            // Rescale the distance between the target and eye so 
            // that it doesn't change. The eye therefore still 
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward + right * self.move_speed).normalize() * forward_mag;
        }
        if self.move_left.get() {
            camera.eye = camera.target - (forward - right * self.move_speed).normalize() * forward_mag;
        }
    }
}


/*
    - cgmath crate is built for OpenGL's coordinate system.
    - this matrix translates and scales our scene from
     OpenGLs coordinate system to WGPU's.
*/
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[derive(Debug)]
pub struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    pub fn new(config: &wgpu::SurfaceConfiguration) -> Self {
        Self {

            //// view ////

            // position the camera one unit up and 2 units back
            // +z is out of the screen
            eye: (0.0, 1.0, 2.0).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),

            //// projection ////

            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        }
    }
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        // 1.
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        // 2.
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        // 3.
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformBuffer {
    // We can't use cgmath with bytemuck directly so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    view_proj: [[f32; 4]; 4],
}

impl UniformBuffer {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}


// projection split from the camera. (deprecated)
#[deprecated]
pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(
        width: u32,
        height: u32,
        fovy: F,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}
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

    rotate_left: Cell<bool>,
    rotate_right : Cell<bool>,
    rotate_up: Cell<bool>,
    rotate_down: Cell<bool>,
    
    rotate_speed: f32,

    scroll: f32,
    sensitivity: f32,
}

/*
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

            rotate_left: Cell::new(false),
            rotate_right : Cell::new(false),
            rotate_up: Cell::new(false),
            rotate_down: Cell::new(false),

            rotate_speed: 1.0,

            scroll: 0.0,
            sensitivity,
        }
    }

    pub fn process_keydown(&self, key: VirtualKeyCode, state: ElementState) {
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

    pub fn process_keyup(&self, key: VirtualKeyCode, state: ElementState) {
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

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        // Move forward/backward and left/right
        let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
        let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
        camera.position += forward * (self.move_forward - self.move_backward) * self.speed * dt;
        camera.position += right * (self.move_right - self.move_left) * self.speed * dt;

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        let (pitch_sin, pitch_cos) = camera.pitch.0.sin_cos();
        let scrollward = Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        camera.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
        self.scroll = 0.0;

        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.
        camera.position.y += (self.move_up - self.move_down) * self.speed * dt;

        // Rotate
        camera.yaw += Rad(self.rotate_horizontal) * self.sensitivity * dt;
        camera.pitch += Rad(-self.rotate_vertical) * self.sensitivity * dt;

        // If process_mouse isn't called every frame, these values
        // will not get set to zero, and the camera will rotate
        // when moving in a non cardinal direction.
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        // Keep the camera's angle from going too high/low.
        if camera.pitch < -Rad(SAFE_FRAC_PI_2) {
            camera.pitch = -Rad(SAFE_FRAC_PI_2);
        } else if camera.pitch > Rad(SAFE_FRAC_PI_2) {
            camera.pitch = Rad(SAFE_FRAC_PI_2);
        }
    }
}
*/

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
struct CameraUniform {
    // We can't use cgmath with bytemuck directly so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}


// projection split from the camera.
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
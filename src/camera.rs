//! First-person camera and movement logic.
//!
//! The `Camera` tracks position, yaw, and pitch and builds a macroquad
//! `Camera3D` each frame. Uses Y as the up axis to match macroquad's
//! `draw_cube` model orientation. Movement input is accumulated as a
//! direction vector and applied each frame alongside physics (gravity) from
//! `physics.rs`.

use crate::input::Input;
use macroquad::prelude::*;

/// Field of view in degrees.
pub const FOV: f32 = 60.0;

/// Near clipping plane distance.
pub const NEAR: f32 = 0.1;

/// Far clipping plane distance.
pub const FAR: f32 = 1000.0;

/// Movement speed in world units per second.
pub const MOVE_SPEED: f32 = 5.0;

/// Mouse sensitivity (yaw/pitch change per normalized delta unit).
pub const MOUSE_SENSITIVITY: f32 = 80.0;

/// A first-person camera.
pub struct Camera {
    /// Position in world space.
    pub position: Vec3,
    /// Yaw angle in degrees (rotation around Y axis; 0 = looking down +Z).
    pub yaw: f32,
    /// Pitch angle in degrees (rotation around X axis; 0 = horizontal).
    pub pitch: f32,
    /// Accumulated movement direction for the current frame.
    pub move_direction: Vec3,
}

impl Camera {
    /// Creates a new camera at the given position looking toward the origin.
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            yaw: 0.0,
            pitch: -15.0,
            move_direction: Vec3::ZERO,
        }
    }

    /// Returns the forward direction vector derived from yaw and pitch.
    ///
    /// Y is up; yaw rotates around Y (0 = looking down +Z), pitch rotates
    /// around X (positive = looking up).
    pub fn forward(&self) -> Vec3 {
        let (yaw, pitch) = (self.yaw.to_radians(), self.pitch.to_radians());
        Vec3::new(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            yaw.cos() * pitch.cos(),
        )
        .normalize()
    }

    /// Returns the right direction vector derived from yaw.
    ///
    /// Horizontal (no pitch tilt), perpendicular to `forward`, so A/D strafe
    /// left/right relative to where the camera is facing.
    pub fn right(&self) -> Vec3 {
        let f = self.forward();
        Vec3::new(-f.z, 0.0, f.x).normalize()
    }

    /// Returns the up vector (world up, Y axis).
    pub fn up(&self) -> Vec3 {
        Vec3::Y
    }

    /// Builds a macroquad `Camera3D` for this camera at the given aspect ratio.
    pub fn camera3d(&self, aspect: f32) -> Camera3D {
        Camera3D {
            position: self.position,
            up: self.up(),
            target: self.position + self.forward(),
            fovy: FOV.to_radians(),
            aspect: Some(aspect),
            projection: Projection::Perspective,
            render_target: None,
            viewport: None,
            z_near: NEAR,
            z_far: FAR,
        }
    }

    /// Queues movement along the camera's forward/right axes.
    ///
    /// `forward` is positive when moving forward, `right` is positive when
    /// strafing right. Both are accumulated into `move_direction`.
    pub fn add_movement(&mut self, forward: f32, right: f32) {
        let f = self.forward();
        let r = self.right();
        // Flatten movement to the horizontal (X-Z) plane.
        let flat_forward = Vec3::new(f.x, 0.0, f.z).normalize();
        let flat_right = Vec3::new(r.x, 0.0, r.z).normalize();
        self.move_direction += flat_forward * forward + flat_right * right;
    }
}

/// Applies keyboard and mouse input to the camera for one frame.
pub fn update_camera_input(camera: &mut Camera, input: &Input) {
    // Only rotate the camera when the cursor is captured (first-person look).
    // When released (ESC), mouse movement does not affect the view.
    if input.cursor_captured {
        // Camera rotation from mouse delta.
        // macroquad's mouse_delta_position() returns (last - current), so yaw
        // must be negated to match natural FPS look direction.
        let (dx, dy) = input.mouse_delta();
        camera.yaw += dx * MOUSE_SENSITIVITY;
        camera.pitch += dy * MOUSE_SENSITIVITY;
        camera.pitch = camera.pitch.clamp(-89.0, 89.0);
    }

    // Movement axes from held keys.
    camera.add_movement(input.forward_axis(), input.right_axis());

    // Apply cursor grab state.
    set_cursor_grab(input.cursor_captured);
    show_mouse(!input.cursor_captured);
}

impl Default for Camera {
    fn default() -> Self {
        // Spawn on top of the grass surface (y=4): feet at y=4, so camera eye
        // at y = 4 + 1.0 (block top) + 1.62 (eye offset) = 6.62. Center of
        // the 16x16 field at x=8, z=8.
        Self::new(Vec3::new(8.0, 6.62, 8.0))
    }
}

//! Input state tracking.
//!
//! Reads macroquad's immediate-mode input each frame: held movement keys,
//! relative mouse motion for camera look, and cursor-capture state.

use macroquad::prelude::*;

/// Tracks current input state read directly from macroquad each frame.
pub struct Input {
    /// Whether the cursor is captured (locked to the window for looking).
    pub cursor_captured: bool,
}

impl Input {
    pub fn new() -> Self {
        Self {
            cursor_captured: true,
        }
    }

    /// Returns the net forward/backward movement (-1, 0, or 1).
    pub fn forward_axis(&self) -> f32 {
        (is_key_down(KeyCode::W) as i8 - is_key_down(KeyCode::S) as i8) as f32
    }

    /// Returns the net left/right movement (-1, 0, or 1).
    pub fn right_axis(&self) -> f32 {
        (is_key_down(KeyCode::D) as i8 - is_key_down(KeyCode::A) as i8) as f32
    }

    /// Returns whether jump was requested this frame.
    pub fn jump_pressed(&self) -> bool {
        is_key_pressed(KeyCode::Space)
    }

    /// Returns whether the escape key was pressed this frame.
    pub fn escape_pressed(&self) -> bool {
        is_key_pressed(KeyCode::Escape)
    }

    /// Returns whether the left mouse button was pressed this frame.
    pub fn left_mouse_pressed(&self) -> bool {
        is_mouse_button_pressed(MouseButton::Left)
    }

    /// Returns whether the right mouse button was pressed this frame.
    pub fn right_mouse_pressed(&self) -> bool {
        is_mouse_button_pressed(MouseButton::Right)
    }

    /// Returns whether any mouse button was pressed this frame.
    pub fn mouse_pressed(&self) -> bool {
        is_mouse_button_pressed(MouseButton::Left)
            || is_mouse_button_pressed(MouseButton::Right)
            || is_mouse_button_pressed(MouseButton::Middle)
    }

    /// Returns the relative mouse motion since the last frame (x, y).
    pub fn mouse_delta(&self) -> (f32, f32) {
        let d = mouse_delta_position();
        (d.x, d.y)
    }
}

//! Physics and gravity integration.
//!
//! This module applies gravity to the camera's vertical velocity each frame,
//! resolves movement against the world, and handles ground detection so the
//! player can walk and fall. Uses Y as the up axis.

use crate::block::BlockType;
use crate::camera::Camera;
use crate::camera::MOVE_SPEED;
use crate::world::World;
use macroquad::prelude::*;

/// Strength of gravity (world units per second squared).
pub const GRAVITY: f32 = -20.0;

/// Jump impulse applied when the player jumps while grounded.
pub const JUMP_SPEED: f32 = 7.0;

/// Terminal fall speed cap.
pub const TERMINAL_VELOCITY: f32 = -40.0;

/// The player's physics state.
pub struct Physics {
    /// Current vertical velocity (positive = moving up).
    pub velocity_y: f32,
    /// Whether the player is currently standing on a solid block.
    pub grounded: bool,
    /// Coyote time timer: frames remaining to jump after walking off a ledge.
    pub jump_grace: f32,
    /// Player collision radius (horizontal).
    pub radius: f32,
    /// Player height (vertical).
    pub height: f32,
    /// Offset from camera position to the player's feet.
    pub eye_offset: f32,
}

impl Physics {
    pub fn new() -> Self {
        Self {
            velocity_y: 0.0,
            grounded: false,
            jump_grace: 0.0,
            radius: 0.35,
            height: 1.8,
            eye_offset: 1.62,
        }
    }

    /// Applies gravity and integrates vertical motion over `dt` seconds.
    pub fn integrate(&mut self, dt: f32) {
        self.velocity_y += GRAVITY * dt;
        if self.velocity_y < TERMINAL_VELOCITY {
            self.velocity_y = TERMINAL_VELOCITY;
        }
        // Count down coyote time so jumping right after walking off a ledge
        // still works for a few frames.
        if self.jump_grace > 0.0 {
            self.jump_grace -= dt;
        }
    }

    /// Applies a jump impulse if the player is grounded (or within coyote time).
    pub fn jump(&mut self) {
        if self.grounded || self.jump_grace > 0.0 {
            self.velocity_y = JUMP_SPEED;
            self.grounded = false;
            self.jump_grace = 0.0;
        }
    }

    /// Resolves vertical movement against the world.
    ///
    /// When falling, checks the block directly beneath the feet. If solid,
    /// snaps the feet to the top of that block and resets velocity. When
    /// rising, moves freely (head collision is intentionally ignored for
    /// simplicity).
    pub fn resolve_vertical(&mut self, camera: &mut Camera, world: &World, dt: f32) {
        let next_y = camera.position.y + self.velocity_y * dt;
        let feet_y = next_y - self.eye_offset;

        if self.velocity_y < 0.0 {
            // Falling: check the block directly below the feet. When feet are
            // exactly on top of a block (e.g. y=5.0 on grass at y=4),
            // floor(feet_y) gives the air block above, so check both the block
            // at feet level and one below.
            let below_y = feet_y.floor() as i32;
            let cx = camera.position.x as i32;
            let cz = camera.position.z as i32;

            let block_at_feet = world.get_block(cx, below_y, cz);
            let block_below = world.get_block(cx, below_y - 1, cz);

            if block_at_feet.is_solid()
                || (block_below.is_solid() && feet_y <= below_y as f32 + 0.1)
            {
                // Land: place feet exactly on top of the highest solid block
                // beneath the feet.
                let land_y = if block_at_feet.is_solid() {
                    below_y as f32
                } else {
                    (below_y - 1) as f32
                };
                camera.position.y = land_y + 1.0 + self.eye_offset;
                self.velocity_y = 0.0;
                self.grounded = true;
                self.jump_grace = 0.15; // 150ms coyote time
            } else {
                camera.position.y = next_y;
                self.grounded = false;
            }
        } else {
            // Rising or stationary: move freely.
            camera.position.y = next_y;
            self.grounded = false;
        }
    }

    /// Resolves horizontal movement against the world using the camera's
    /// accumulated move direction. X and Z are resolved independently so the
    /// player slides along walls instead of stopping completely.
    pub fn resolve_horizontal(&mut self, camera: &mut Camera, world: &World, dt: f32) {
        if camera.move_direction.length_squared() == 0.0 {
            return;
        }

        let move_dir = camera.move_direction.normalize();
        let delta = move_dir * MOVE_SPEED * dt;

        let eye_y = camera.position.y - self.eye_offset;
        let bottom = eye_y + 0.1;
        let top = eye_y + self.height - 0.1;

        // Try X movement.
        let next_x = camera.position.x + delta.x;
        if !self.collides_at(world, next_x, camera.position.z, bottom, top) {
            camera.position.x = next_x;
        }

        // Try Z movement independently (sliding along walls).
        let next_z = camera.position.z + delta.z;
        if !self.collides_at(world, camera.position.x, next_z, bottom, top) {
            camera.position.z = next_z;
        }

        camera.move_direction = Vec3::ZERO;
    }

    /// Returns true if the player's bounding box at the given (x, z)
    /// position overlaps any solid block between `bottom` and `top` in Y.
    fn collides_at(&self, world: &World, x: f32, z: f32, bottom: f32, top: f32) -> bool {
        let min_x = (x - self.radius) as i32;
        let max_x = (x + self.radius) as i32;
        let min_z = (z - self.radius) as i32;
        let max_z = (z + self.radius) as i32;
        let min_y = bottom.floor() as i32;
        let max_y = top.floor() as i32;

        for cx in min_x..=max_x {
            for cy in min_y..=max_y {
                for cz in min_z..=max_z {
                    if world.get_block(cx, cy, cz) == BlockType::Air {
                        continue;
                    }
                    return true;
                }
            }
        }
        false
    }
}

impl Default for Physics {
    fn default() -> Self {
        Self::new()
    }
}

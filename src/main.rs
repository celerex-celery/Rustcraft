//! Application entry point.
//!
//! Runs the macroquad main loop. Each frame: read input, update the camera,
//! apply physics (gravity + movement), load nearby chunks, and render the
//! voxel world.

use crate::block::BlockType;
use crate::camera::{update_camera_input, Camera};
use crate::input::Input;
use crate::physics::Physics;
use crate::renderer::Renderer;
use crate::world::World;
use macroquad::prelude::*;
use windows::Win32::System::ProcessStatus::{
    GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
};
use windows::Win32::System::Threading::GetCurrentProcess;

mod block;
mod camera;
mod chunk;
mod frustum;
mod input;
mod physics;
mod renderer;
mod world;

const MAX_REACH: f32 = 12.0;
const STEP_SIZE: f32 = 0.1;

struct App {
    world: World,
    camera: Camera,
    physics: Physics,
    input: Input,
    renderer: Option<Renderer>,
    frame_count: u32,
    fps_timer: f32,
    fps: u32,
}

/// Casts a ray from the camera position along the forward direction and
/// returns the block coordinate (x, y, z) of the first solid block hit,
/// or None if nothing is within reach.
fn raycast_block(world: &World, camera: &Camera) -> Option<(i32, i32, i32)> {
    let origin = camera.position;
    let dir = camera.forward();

    let mut t = 0.0;
    while t < MAX_REACH {
        let pos = origin + dir * t;
        let bx = pos.x as i32;
        let by = pos.y as i32;
        let bz = pos.z as i32;

        if world.get_block(bx, by, bz) != BlockType::Air {
            return Some((bx, by, bz));
        }

        t += STEP_SIZE;
    }
    None
}

/// Window configuration: 1920x1080 full HD.
fn window_conf() -> macroquad::conf::Conf {
    macroquad::conf::Conf {
        miniquad_conf: macroquad::miniquad::conf::Conf {
            window_title: "Minexel".to_string(),
            window_width: 1920,
            window_height: 1080,
            icon: None,
            ..Default::default()
        },
        ..Default::default()
    }
}

impl App {
    fn new() -> Self {
        Self {
            world: World::new(),
            camera: Camera::new(Vec3::new(64.0, 52.0, 64.0)),
            physics: Physics::new(),
            input: Input::new(),
            renderer: None,
            frame_count: 0,
            fps_timer: 0.0,
            fps: 0,
        }
    }

    /// Reads input, applies physics and movement to the camera, and loads
    /// chunks around the player.
    fn update(&mut self) {
        let dt = get_frame_time();

        // FPS tracking.
        self.frame_count += 1;
        self.fps_timer += dt;
        if self.fps_timer >= 1.0 {
            self.fps = self.frame_count;
            self.frame_count = 0;
            self.fps_timer = 0.0;
        }

        // Look / movement input (also applies movement direction).
        update_camera_input(&mut self.camera, &self.input);

        // Load/unload chunks around the player based on render distance.
        self.world.update_chunks_around_player(
            self.camera.position.x,
            self.camera.position.z,
        );

        // Toggle cursor capture on Escape.
        if self.input.escape_pressed() {
            self.input.cursor_captured = !self.input.cursor_captured;
        }

        // Re-grab cursor on any mouse click so pointer lock returns.
        if self.input.mouse_pressed() {
            self.input.cursor_captured = true;
        }

        // Jump.
        if self.input.jump_pressed() {
            self.physics.jump();
        }

        // Break block under crosshair on left click.
        if self.input.left_mouse_pressed() && self.input.cursor_captured {
            if let Some((bx, by, bz)) = raycast_block(&self.world, &self.camera) {
                self.world.set_block(bx, by, bz, BlockType::Air);
                // Rebuild chunk mesh so the hole appears.
                let cx = bx.div_euclid(16);
                let cz = bz.div_euclid(16);
                if let Some(chunk) = self.world.get_chunk_mut(cx, cz) {
                    chunk.build_mesh();
                }
            }
        }

        // Place stone under crosshair on right click.
        if self.input.right_mouse_pressed() && self.input.cursor_captured {
            let origin = self.camera.position;
            let dir = self.camera.forward();
            let mut t = 0.0;
            let mut last_air: Option<(i32, i32, i32)> = None;

            while t < MAX_REACH {
                let pos = origin + dir * t;
                let bx = pos.x as i32;
                let by = pos.y as i32;
                let bz = pos.z as i32;

                if self.world.get_block(bx, by, bz) != BlockType::Air {
                    // Hit a solid block: place stone on the last air block
                    // we passed (the face of the solid block we're looking at).
                    if let Some((px, py, pz)) = last_air {
                        self.world.set_block(px, py, pz, BlockType::Stone);
                        let cx = px.div_euclid(16);
                        let cz = pz.div_euclid(16);
                        if let Some(chunk) = self.world.get_chunk_mut(cx, cz) {
                            chunk.build_mesh();
                        }
                    }
                    break;
                }

                last_air = Some((bx, by, bz));
                t += STEP_SIZE;
            }
        }

        // Physics: gravity + vertical + horizontal resolution.
        self.physics.integrate(dt);
        self.physics.resolve_vertical(&mut self.camera, &self.world, dt);
        self.physics.resolve_horizontal(&mut self.camera, &self.world, dt);

        // Keep the camera from falling through the world (absolute floor at Y=0).
        if self.camera.position.y < 0.0 {
            self.camera.position.y = 48.0;
            self.physics.velocity_y = 0.0;
            self.physics.grounded = true;
        }

        // The world is a single fixed 16x16 chunk; no on-demand loading needed.
    }

    /// Renders the world.
    fn render(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            renderer.render(&self.world, &self.camera);
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Request the cursor be captured for first-person look.
    set_cursor_grab(true);
    show_mouse(false);

    let mut app = App::new();

    // Initialize the renderer (loads textures) asynchronously.
    match Renderer::new().await {
        Ok(renderer) => app.renderer = Some(renderer),
        Err(e) => {
            eprintln!("Failed to initialize renderer: {}", e);
            return;
        }
    }

    loop {
        clear_background(BLUE); // sky color

        app.update();

        app.render();

        // Ensure cursor is grabbed every frame if requested (prevents escape).
        if app.input.cursor_captured {
            set_cursor_grab(true);
            show_mouse(false);
        }

        // Read this process's working set via Windows API so it matches
        // Task Manager's memory column.
        let mut counters: PROCESS_MEMORY_COUNTERS = PROCESS_MEMORY_COUNTERS::default();
        counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        unsafe {
            GetProcessMemoryInfo(
                GetCurrentProcess(),
                &mut counters,
                counters.cb,
            )
            .ok();
        }
        let ram_used_mb = counters.WorkingSetSize / 1024 / 1024;
        let ram_total_mb = unsafe {
            let mut mi: windows::Win32::System::SystemInformation::MEMORYSTATUSEX =
                windows::Win32::System::SystemInformation::MEMORYSTATUSEX::default();
            mi.dwLength = std::mem::size_of::<windows::Win32::System::SystemInformation::MEMORYSTATUSEX>() as u32;
            windows::Win32::System::SystemInformation::GlobalMemoryStatusEx(&mut mi).ok();
            mi.ullTotalPhys / 1024 / 1024
        };

        // Debug overlay: show position, FPS, RAM, and controls.
        draw_text(
            &format!(
                "POS: {:.1} {:.1} {:.1} | FPS: {} | RAM: {} MB / {} MB",
                app.camera.position.x,
                app.camera.position.y,
                app.camera.position.z,
                app.fps,
                ram_used_mb,
                ram_total_mb
            ),
            10.0,
            20.0,
            20.0,
            WHITE,
        );

        next_frame().await
    }
}

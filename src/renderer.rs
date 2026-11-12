//! Rendering pipeline built on macroquad.
//!
//! This module loads the block textures (grass side, grass top, dirt, stone)
//! and renders every visible cube face individually with the correct texture.
//! Per-face texture mapping:
//!   * Grass blocks: grass_side on sides, grass_top on top, dirt on bottom.
//!   * Dirt blocks: dirt on all faces.
//!   * Stone blocks: stone on all faces.
//!
//! The chunk's face-culled mesh (only faces adjacent to air) is used, so
//! hidden faces between adjacent solid blocks are never drawn. Basic
//! directional lighting shades each face by its surface normal.

use crate::block::{BlockType, CHUNK_WIDTH, Face};
use crate::camera::Camera;
use crate::world::World;
use macroquad::prelude::*;

pub struct Renderer {
    /// Texture used for grass block sides.
    grass_side_texture: Texture2D,
    /// Texture used for grass block tops.
    grass_top_texture: Texture2D,
    /// Texture used for dirt blocks (and grass block bottoms).
    dirt_texture: Texture2D,
    /// Texture used for stone blocks.
    stone_texture: Texture2D,
    /// Crosshair texture drawn centered on screen.
    crosshair_texture: Texture2D,
}

impl Renderer {
    /// Loads the block textures from the assets directory, loading them raw
    /// (no flipv) so they appear right-side up when paired with macroquad's
    /// UV convention.
    pub async fn new() -> Result<Self, macroquad::Error> {
        let grass_side_texture = load_texture("assets/textures/blocks/grass_block_side.png").await?;
        let grass_top_texture = load_texture("assets/textures/blocks/grass_block_top.png").await?;
        let dirt_texture = load_texture("assets/textures/blocks/dirt.png").await?;
        let stone_texture = load_texture("assets/textures/blocks/stone.png").await?;
        let crosshair_texture = load_texture("assets/textures/gui/crosshair.png").await?;

        // Use nearest-neighbor filtering so the voxel textures stay crisp.
        grass_side_texture.set_filter(FilterMode::Nearest);
        grass_top_texture.set_filter(FilterMode::Nearest);
        dirt_texture.set_filter(FilterMode::Nearest);
        stone_texture.set_filter(FilterMode::Nearest);
        crosshair_texture.set_filter(FilterMode::Nearest);

        Ok(Self {
            grass_side_texture,
            grass_top_texture,
            dirt_texture,
            stone_texture,
            crosshair_texture,
        })
    }

    /// Returns the texture for a specific block face.
    ///
    /// Grass blocks use different textures per face: top gets the grass top
    /// texture, bottom gets dirt, sides get grass side. Dirt blocks use dirt
    /// on every face. Stone blocks use stone on every face.
    fn face_texture(&self, block: BlockType, face: Face) -> &Texture2D {
        match block {
            BlockType::Grass => match face {
                Face::Top => &self.grass_top_texture,
                Face::Bottom => &self.dirt_texture,
                _ => &self.grass_side_texture,
            },
            BlockType::Dirt => &self.dirt_texture,
            BlockType::Stone => &self.stone_texture,
            BlockType::Air => &self.dirt_texture, // unreachable for solid faces
        }
    }

    /// Computes a simple lighting factor from a surface normal.
    ///
    /// Faces angled toward the light are brighter; faces angled away are
    /// darker, with a minimum ambient level so unlit faces remain visible.
    fn lighting_factor(normal: Vec3) -> f32 {
        let light_dir = Vec3::new(0.4, 1.0, 0.3).normalize();
        let normal = normal.normalize();
        normal.dot(light_dir).clamp(0.25, 1.0)
    }

    /// Renders the entire world from the camera's point of view.
    pub fn render(&mut self, world: &World, camera: &Camera) {
        let aspect = screen_width() / screen_height();
        set_camera(&camera.camera3d(aspect));

        // Draw the world (opaque geometry, depth-tested).
        self.render_world(world);

        // Return to the default 2D camera for any UI overlay.
        set_default_camera();
    }

    fn render_world(&self, world: &World) {
        for (_chunk_coord, chunk) in &world.chunks {
            let mesh = chunk.mesh.as_ref().expect("chunk mesh not built");
            let chunk_offset = Vec3::new(
                chunk.position.0 as f32 * CHUNK_WIDTH as f32,
                0.0,
                chunk.position.1 as f32 * CHUNK_WIDTH as f32,
            );

            for mesh_face in mesh.iter() {
                let block = mesh_face.block;
                if !block.is_solid() {
                    continue;
                }

                let face = mesh_face.face;
                let texture = self.face_texture(block, face);
                let world_pos = chunk_offset + Vec3::from(mesh_face.position);
                let normal = Vec3::from(face.normal());

                // Lighting based on this face's normal.
                let light = Self::lighting_factor(normal);

                // Tint grass tops greener so they stand out from dirt sides.
                let color = if block == BlockType::Grass && face == Face::Top {
                    Color {
                        r: light * 0.6,
                        g: (light * 1.5).clamp(0.0, 1.0),
                        b: light * 0.5,
                        a: 1.0,
                    }
                } else {
                    WHITE
                };
                let color = color_to_rgba(color, light);

                draw_cube_face(world_pos, face, texture, color);
            }
        }
    }
}

/// Converts a `Color` and lighting factor into `[u8; 4]` for a `Vertex`.
fn color_to_rgba(base: Color, light: f32) -> [u8; 4] {
    [
        (base.r * 255.0 * light) as u8,
        (base.g * 255.0 * light) as u8,
        (base.b * 255.0 * light) as u8,
        (base.a * 255.0) as u8,
    ]
}

/// Loads a PNG texture from `path`, loading it raw (no flipv) so it appears
/// right-side up on cube faces when paired with macroquad's UV convention.
fn load_texture_flipped(path: &str) -> Result<Texture2D, macroquad::Error> {
    let img = image::open(path)?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(Texture2D::from_rgba8(width as u16, height as u16, &rgba.into_raw()))
}

/// Draws a single cube face as a textured quad in 3D space.
///
/// Builds a temporary `Mesh` with the four corners of the given face,
/// applies per-vertex texture coordinates, and draws it with `draw_mesh`.
/// This enables per-face texturing (e.g. grass top != grass side).
fn draw_cube_face(position: Vec3, face: Face, texture: &Texture2D, color: [u8; 4]) {
    // Face corner offsets and UVs matching macroquad's internal draw_cube
    // convention exactly, so textures are upright on every face.
    // Textures loaded raw (no flipv). Macroquad uses V=0 at top of texture,
    // matching PNG row 0 = top of image. Front/Back use normal UVs;
    // Top/Bottom/Left/Right use inverted V UVs.
    let (corners, uvs): ([Vec3; 4], [Vec2; 4]) = match face {
        Face::Top => (
            [
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 1.0, 1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, 1.0, 0.0),
            ],
            [
                Vec2::new(0.0, 1.0),
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
            ],
        ),
        Face::Bottom => (
            [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, 0.0),
            ],
            [
                Vec2::new(0.0, 1.0),
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
            ],
        ),
        Face::Left => (
            [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 1.0, 1.0),
                Vec3::new(0.0, 0.0, 1.0),
            ],
            [
                Vec2::new(0.0, 1.0),
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
            ],
        ),
        Face::Right => (
            [
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, 0.0, 1.0),
            ],
            [
                Vec2::new(0.0, 1.0),
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
            ],
        ),
        Face::Front => (
            [
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, 1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(0.0, 1.0, 1.0),
            ],
            [
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(0.0, 1.0),
            ],
        ),
        Face::Back => (
            [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            [
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(0.0, 1.0),
            ],
        ),
    };

    let normal: Vec4 = match face {
        Face::Top => Vec4::new(0.0, 1.0, 0.0, 0.0),
        Face::Bottom => Vec4::new(0.0, -1.0, 0.0, 0.0),
        Face::Left => Vec4::new(-1.0, 0.0, 0.0, 0.0),
        Face::Right => Vec4::new(1.0, 0.0, 0.0, 0.0),
        Face::Front => Vec4::new(0.0, 0.0, 1.0, 0.0),
        Face::Back => Vec4::new(0.0, 0.0, -1.0, 0.0),
    };

    let vertices = [
        Vertex {
            position: position + corners[0],
            uv: uvs[0],
            normal,
            color,
        },
        Vertex {
            position: position + corners[1],
            uv: uvs[1],
            normal,
            color,
        },
        Vertex {
            position: position + corners[2],
            uv: uvs[2],
            normal,
            color,
        },
        Vertex {
            position: position + corners[3],
            uv: uvs[3],
            normal,
            color,
        },
    ];

    let indices = [0u16, 1, 2, 0, 2, 3];

    let mesh = Mesh {
        vertices: vertices.to_vec(),
        indices: indices.to_vec(),
        texture: Some(texture.weak_clone()),
    };

    draw_mesh(&mesh);
}

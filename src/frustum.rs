//! Frustum culling for the voxel engine.
//!
//! Extracts the 6 view-frustum planes from the camera's view-projection
//! matrix and tests each chunk's axis-aligned bounding box against them.
//! Chunks entirely outside the frustum are skipped during rendering,
//! dramatically reducing the number of faces drawn per frame.

use crate::camera::Camera;
use crate::chunk::Chunk;
use crate::world::World;
use macroquad::prelude::*;

/// One plane of the view frustum.
///
/// Stored as `normal * point + distance = 0`. A point is "inside" the
/// frustum when `dot(normal, point) + distance >= 0` for all 6 planes.
#[derive(Clone, Copy, Debug)]
struct Plane {
    normal: Vec3,
    distance: f32,
}

impl Plane {
    /// Returns the signed distance from `point` to this plane.
    ///
    /// Positive means the point is on the "inside" side of the plane.
    fn signed_distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

/// The 6 planes of the view frustum.
#[derive(Clone, Copy, Debug)]
pub struct Frustum {
    planes: [Plane; 6],
}

impl Frustum {
    /// Extracts the frustum planes from a view-projection matrix.
    ///
    /// Uses the standard OpenGL/DirectX extraction: each plane is the sum
    /// or difference of two rows of the combined matrix, then normalized.
    fn from_view_projection(vp: Mat4) -> Self {
        let r0 = vp.x_axis;
        let r1 = vp.y_axis;
        let r2 = vp.z_axis;
        let r3 = vp.w_axis;

        let planes = [
            // Left:  row3 + row0
            Plane {
                normal: Vec3::new(r3.x + r0.x, r3.y + r0.y, r3.z + r0.z),
                distance: r3.w + r0.w,
            },
            // Right: row3 - row0
            Plane {
                normal: Vec3::new(r3.x - r0.x, r3.y - r0.y, r3.z - r0.z),
                distance: r3.w - r0.w,
            },
            // Bottom: row3 + row1
            Plane {
                normal: Vec3::new(r3.x + r1.x, r3.y + r1.y, r3.z + r1.z),
                distance: r3.w + r1.w,
            },
            // Top: row3 - row1
            Plane {
                normal: Vec3::new(r3.x - r1.x, r3.y - r1.y, r3.z - r1.z),
                distance: r3.w - r1.w,
            },
            // Near: row3 + row2
            Plane {
                normal: Vec3::new(r3.x + r2.x, r3.y + r2.y, r3.z + r2.z),
                distance: r3.w + r2.w,
            },
            // Far: row3 - row2
            Plane {
                normal: Vec3::new(r3.x - r2.x, r3.y - r2.y, r3.z - r2.z),
                distance: r3.w - r2.w,
            },
        ];

        // Normalize each plane so the normal is unit length.
        let mut normalized = [Plane {
            normal: Vec3::ZERO,
            distance: 0.0,
        }; 6];
        for (i, p) in planes.iter().enumerate() {
            let len = p.normal.length();
            if len > 0.0 {
                normalized[i] = Plane {
                    normal: p.normal / len,
                    distance: p.distance / len,
                };
            } else {
                normalized[i] = *p;
            }
        }

        Self { planes: normalized }
    }

    /// Returns `true` if the axis-aligned box `[min, max]` is at least
    /// partially inside the frustum.
    ///
    /// For each frustum plane, if ALL 8 corners of the box are outside
    /// (behind) that plane, the box is completely outside the frustum and
    /// should be culled.
    fn contains_aabb(&self, min: Vec3, max: Vec3) -> bool {
        // All 8 corners of the AABB.
        let corners = [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(max.x, max.y, max.z),
        ];

        for plane in self.planes.iter() {
            // If every corner is outside (negative signed distance), the box
            // is completely outside this plane and should be culled.
            let mut all_outside = true;
            for corner in corners.iter() {
                if plane.signed_distance(*corner) >= 0.0 {
                    all_outside = false;
                    break;
                }
            }
            if all_outside {
                return false;
            }
        }
        true
    }
}

/// Computes the axis-aligned bounding box of a chunk in world space.
fn chunk_aabb(chunk: &Chunk) -> (Vec3, Vec3) {
    let min = Vec3::new(
        chunk.position.0 as f32 * crate::block::CHUNK_WIDTH as f32,
        0.0,
        chunk.position.1 as f32 * crate::block::CHUNK_WIDTH as f32,
    );
    let max = min + Vec3::new(
        crate::block::CHUNK_WIDTH as f32,
        crate::block::CHUNK_HEIGHT as f32,
        crate::block::CHUNK_WIDTH as f32,
    );
    (min, max)
}

/// Builds a `Frustum` from a `Camera` at the given aspect ratio.
pub fn frustum_from_camera(camera: &Camera, aspect: f32) -> Frustum {
    let cam = camera.camera3d(aspect);
    let view = Mat4::look_at_rh(
        Vec3::new(cam.position.x, cam.position.y, cam.position.z),
        Vec3::new(cam.target.x, cam.target.y, cam.target.z),
        Vec3::new(cam.up.x, cam.up.y, cam.up.z),
    );
    let proj = Mat4::perspective_lh(
        cam.fovy,
        cam.aspect.unwrap_or(aspect),
        cam.z_near,
        cam.z_far,
    );
    let vp = proj * view;
    Frustum::from_view_projection(vp)
}

/// Culls chunks against the frustum, returning only the visible ones.
pub fn cull_chunks<'a>(
    frustum: &Frustum,
    world: &'a World,
) -> Vec<&'a crate::chunk::Chunk> {
    let mut visible = Vec::new();
    for (_coord, chunk) in &world.chunks {
        let (min, max) = chunk_aabb(chunk);
        if frustum.contains_aabb(min, max) {
            visible.push(chunk);
        }
    }
    visible
}

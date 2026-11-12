//! Chunk-based world storage and mesh generation.
//!
//! A `Chunk` owns a fixed 16x16x64 grid of `BlockType` values. The mesh is a
//! list of visible cube faces: for each solid block, only the faces bordering
//! a transparent (air) block are emitted (basic face culling).

use crate::block::{BlockType, CHUNK_BLOCK_COUNT, CHUNK_DEPTH, CHUNK_HEIGHT, CHUNK_WIDTH, Face, MeshFace};

/// A single chunk of the voxel world.
///
/// Blocks are stored in a flat array indexed as
/// `(x + y * CHUNK_WIDTH + z * CHUNK_WIDTH * CHUNK_HEIGHT)`.
pub struct Chunk {
    /// The block data for this chunk.
    pub blocks: [BlockType; CHUNK_BLOCK_COUNT],
    /// Cached generated mesh (visible faces). `None` until `build_mesh` runs.
    pub mesh: Option<Vec<MeshFace>>,
    /// World-space chunk coordinate (x, z).
    pub position: (i32, i32),
}

impl Chunk {
    /// Creates a new empty chunk filled with `BlockType::Air`.
    pub fn new(chunk_x: i32, chunk_z: i32) -> Self {
        Self {
            blocks: [BlockType::Air; CHUNK_BLOCK_COUNT],
            mesh: None,
            position: (chunk_x, chunk_z),
        }
    }

    /// Returns the flat-array index for a local coordinate, or `None` if out of bounds.
    pub fn index(x: i32, y: i32, z: i32) -> Option<usize> {
        if x < 0 || x >= CHUNK_WIDTH || y < 0 || y >= CHUNK_HEIGHT || z < 0 || z >= CHUNK_DEPTH {
            return None;
        }
        Some((x + y * CHUNK_WIDTH + z * CHUNK_WIDTH * CHUNK_HEIGHT) as usize)
    }

    /// Returns the block at the given local coordinate, or `Air` if out of bounds.
    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockType {
        match Self::index(x, y, z) {
            Some(idx) => self.blocks[idx],
            None => BlockType::Air,
        }
    }

    /// Sets the block at the given local coordinate and invalidates the mesh.
    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockType) {
        if let Some(idx) = Self::index(x, y, z) {
            self.blocks[idx] = block;
            self.mesh = None;
        }
    }

    /// Builds the visible-face mesh for this chunk.
    ///
    /// For every solid block, each of its six faces is emitted only if the
    /// neighboring block across that face is transparent (air). The resulting
    /// face list is cached in `self.mesh`.
    pub fn build_mesh(&mut self) {
        let mut faces: Vec<MeshFace> = Vec::new();

        for z in 0..CHUNK_DEPTH {
            for y in 0..CHUNK_HEIGHT {
                for x in 0..CHUNK_WIDTH {
                    let block = self.blocks[Self::index(x, y, z).unwrap()];
                    if !block.is_solid() {
                        continue;
                    }

                    for face in [
                        Face::Top,
                        Face::Bottom,
                        Face::Left,
                        Face::Right,
                        Face::Front,
                        Face::Back,
                    ] {
                        let (nx, ny, nz) = self.neighbor_coord(x, y, z, face);
                        let neighbor = self.get(nx, ny, nz);
                        if neighbor.is_solid() {
                            continue;
                        }

                        faces.push(MeshFace {
                            position: [x as f32, y as f32, z as f32],
                            face,
                            block,
                        });
                    }
                }
            }
        }

        self.mesh = Some(faces);
    }

    /// Returns the neighbor block coordinate across the given face.
    fn neighbor_coord(&self, x: i32, y: i32, z: i32, face: Face) -> (i32, i32, i32) {
        match face {
            Face::Top => (x, y + 1, z),
            Face::Bottom => (x, y - 1, z),
            Face::Left => (x - 1, y, z),
            Face::Right => (x + 1, y, z),
            Face::Front => (x, y, z + 1),
            Face::Back => (x, y, z - 1),
        }
    }

    /// Returns a reference to the cached mesh, building it first if needed.
    pub fn ensure_mesh(&mut self) -> &Vec<MeshFace> {
        if self.mesh.is_none() {
            self.build_mesh();
        }
        self.mesh.as_ref().unwrap()
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

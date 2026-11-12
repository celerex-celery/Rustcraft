//! Block type definitions for the voxel engine.
//!
//! Defines the distinct block types (Air, Grass, Dirt) and their visual
//! properties. Grass is the top surface layer; dirt sits directly beneath it.

/// The set of block types supported by the engine.
///
/// `Air` is empty space and is never rendered. `Grass` forms the top layer,
/// `Dirt` forms the layer directly beneath it, and `Stone` fills everything
/// below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Air = 0,
    Grass = 1,
    Dirt = 2,
    Stone = 3,
}

impl BlockType {
    /// Returns `true` if this block is solid and occludes neighbors.
    pub fn is_solid(self) -> bool {
        matches!(self, BlockType::Grass | BlockType::Dirt | BlockType::Stone)
    }
}

impl Default for BlockType {
    fn default() -> Self {
        BlockType::Air
    }
}

/// The six faces of a cube, used for face culling and mesh generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Top,
    Bottom,
    Left,
    Right,
    Front,
    Back,
}

impl Face {
    /// Returns the unit normal vector for this face.
    pub fn normal(self) -> [f32; 3] {
        match self {
            Face::Top => [0.0, 1.0, 0.0],
            Face::Bottom => [0.0, -1.0, 0.0],
            Face::Left => [-1.0, 0.0, 0.0],
            Face::Right => [1.0, 0.0, 0.0],
            Face::Front => [0.0, 0.0, 1.0],
            Face::Back => [0.0, 0.0, -1.0],
        }
    }
}

/// A single visible face in a chunk's mesh.
///
/// Stored per-face so the renderer can draw each visible cube face with the
/// appropriate texture and simple lighting.
#[derive(Clone, Copy, Debug)]
pub struct MeshFace {
    /// Lower-left corner of the block in world space.
    pub position: [f32; 3],
    /// Which face of the block this is.
    pub face: Face,
    /// The block type this face belongs to (selects the texture).
    pub block: BlockType,
}

pub const CHUNK_WIDTH: i32 = 16;
pub const CHUNK_HEIGHT: i32 = 64;
pub const CHUNK_DEPTH: i32 = 16;

/// Total number of blocks in a single chunk.
pub const CHUNK_BLOCK_COUNT: usize = (CHUNK_WIDTH * CHUNK_HEIGHT * CHUNK_DEPTH) as usize;

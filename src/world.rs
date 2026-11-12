//! World generation and chunk management.
//!
//! The `World` owns a sparse grid of chunks keyed by their (x, z) chunk
//! coordinate. Chunks are loaded and unloaded dynamically based on player
//! position with a render distance, so only nearby chunks exist in memory.
//!
//! Heightmap with hills:
//!   * Grass: surface height varies per (x, z) via noise (Y=38..46)
//!   * Stone: below grass down to Y=0
//!   * Air: above grass up to Y=63

use crate::block::{BlockType, CHUNK_HEIGHT, CHUNK_WIDTH};
use crate::chunk::Chunk;
use std::collections::HashMap;

/// Number of chunks loaded around the player in each axis.
const RENDER_DISTANCE_CHUNKS: i32 = 1;

/// Number of chunks across in each axis (world size in chunks).
const WORLD_CHUNKS_ACROSS: i32 = 16;

/// Base surface height (sea level).
const SEA_LEVEL: i32 = 40;

/// Maximum hill height variation above/below sea level.
const HILL_AMPLITUDE: i32 = 6;

/// Bottom-most height (inclusive) that stone fills.
const STONE_BOTTOM: i32 = 0;

/// The voxel world, managing a dynamic grid of chunks.
pub struct World {
    /// Chunks keyed by their (x, z) chunk coordinate.
    pub chunks: HashMap<(i32, i32), Chunk>,
}

impl World {
    /// Creates a new world with no chunks loaded yet. Chunks are loaded
    /// on-demand as the player moves around.
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }

    /// Loads the chunk at the given chunk coordinate if it is not already
    /// loaded. Generates a fresh heightmap chunk with rolling hills.
    pub fn load_chunk(&mut self, chunk_x: i32, chunk_z: i32) {
        if self.chunks.contains_key(&(chunk_x, chunk_z)) {
            return;
        }

        let mut chunk = Chunk::new(chunk_x, chunk_z);

        for z in 0..CHUNK_WIDTH {
            for x in 0..CHUNK_WIDTH {
                // World-space block coordinate.
                let wx = chunk_x * CHUNK_WIDTH as i32 + x;
                let wz = chunk_z * CHUNK_WIDTH as i32 + z;

                // Compute surface height via noise (hills).
                let surface_y = surface_height(wx, wz);

                // Place grass at surface.
                chunk.set(x, surface_y, z, BlockType::Grass);

                // Place stone below grass down to bottom.
                for y in STONE_BOTTOM..surface_y {
                    chunk.set(x, y, z, BlockType::Stone);
                }
            }
        }

        chunk.build_mesh();
        self.chunks.insert((chunk_x, chunk_z), chunk);
    }

    /// Unloads the chunk at the given coordinate if it exists.
    pub fn unload_chunk(&mut self, chunk_x: i32, chunk_z: i32) {
        self.chunks.remove(&(chunk_x, chunk_z));
    }

    /// Returns the chunk at the given coordinate, or `None` if not loaded.
    pub fn get_chunk(&self, chunk_x: i32, chunk_z: i32) -> Option<&Chunk> {
        self.chunks.get(&(chunk_x, chunk_z))
    }

    /// Returns a mutable reference to the chunk at the given coordinate.
    pub fn get_chunk_mut(&mut self, chunk_x: i32, chunk_z: i32) -> Option<&mut Chunk> {
        self.chunks.get_mut(&(chunk_x, chunk_z))
    }

    /// Returns the block type at a world coordinate, or `Air` if the chunk is
    /// not loaded or the coordinate is out of range.
    pub fn get_block(&self, world_x: i32, world_y: i32, world_z: i32) -> BlockType {
        if world_y < 0 || world_y >= CHUNK_HEIGHT {
            return BlockType::Air;
        }
        let chunk_x = world_x.div_euclid(CHUNK_WIDTH);
        let chunk_z = world_z.div_euclid(CHUNK_WIDTH);
        let local_x = world_x.rem_euclid(CHUNK_WIDTH);
        let local_z = world_z.rem_euclid(CHUNK_WIDTH);

        match self.get_chunk(chunk_x, chunk_z) {
            Some(chunk) => chunk.get(local_x, world_y, local_z),
            None => BlockType::Air,
        }
    }

    /// Sets the block type at a world coordinate. Does nothing if outside
    /// vertical range or chunk is not loaded.
    pub fn set_block(&mut self, world_x: i32, world_y: i32, world_z: i32, block: BlockType) {
        if world_y < 0 || world_y >= CHUNK_HEIGHT {
            return;
        }
        let chunk_x = world_x.div_euclid(CHUNK_WIDTH);
        let chunk_z = world_z.div_euclid(CHUNK_WIDTH);
        let local_x = world_x.rem_euclid(CHUNK_WIDTH);
        let local_z = world_z.rem_euclid(CHUNK_WIDTH);

        if let Some(chunk) = self.get_chunk_mut(chunk_x, chunk_z) {
            chunk.set(local_x, world_y, local_z, block);
        }
    }

    /// Loads chunks around the player's current position and unloads chunks
    /// that are too far away. Call this every frame.
    pub fn update_chunks_around_player(&mut self, player_x: f32, player_z: f32) {
        let player_chunk_x = (player_x / CHUNK_WIDTH as f32).floor() as i32;
        let player_chunk_z = (player_z / CHUNK_WIDTH as f32).floor() as i32;

        // Load all chunks within render distance.
        for cz in (player_chunk_z - RENDER_DISTANCE_CHUNKS)
            ..=(player_chunk_z + RENDER_DISTANCE_CHUNKS)
        {
            for cx in (player_chunk_x - RENDER_DISTANCE_CHUNKS)
                ..=(player_chunk_x + RENDER_DISTANCE_CHUNKS)
            {
                self.load_chunk(cx, cz);
            }
        }

        // Unload chunks that are too far from the player.
        let to_unload: Vec<(i32, i32)> = self
            .chunks
            .keys()
            .filter(|(cx, cz)| {
                (cx - player_chunk_x).abs() > RENDER_DISTANCE_CHUNKS + 1
                    || (cz - player_chunk_z).abs() > RENDER_DISTANCE_CHUNKS + 1
            })
            .copied()
            .collect();

        for (cx, cz) in to_unload {
            self.unload_chunk(cx, cz);
        }
    }
}

/// Simple pseudo-random noise using sine/cosine combinations.
/// Returns a value in [-1.0, 1.0].
fn noise(x: i32, z: i32) -> f32 {
    let xf = x as f32;
    let zf = z as f32;
    let n1 = (xf * 0.05).sin() * (zf * 0.05).cos();
    let n2 = (xf * 0.13 + zf * 0.07).sin() * 0.5;
    let n3 = (xf * 0.03 - zf * 0.09).cos() * 0.3;
    (n1 + n2 + n3) / 1.8
}

/// Returns the surface height at world coordinate (x, z).
/// Height ranges from SEA_LEVEL - HILL_AMPLITUDE to SEA_LEVEL + HILL_AMPLITUDE.
fn surface_height(x: i32, z: i32) -> i32 {
    let n = noise(x, z);
    let height = SEA_LEVEL as f32 + n * HILL_AMPLITUDE as f32;
    height.clamp(2.0, 60.0) as i32
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

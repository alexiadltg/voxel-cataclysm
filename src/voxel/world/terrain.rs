use bevy::{
    prelude::{Added, Local, Plugin, Query, Res, ResMut, StageLabel, SystemStage},
    tasks::ComputeTaskPool,
};

use super::{
    chunks::{ChunkLoadingStage, DirtyChunks},
    Chunk, ChunkKey, ChunkShape, Voxel, CHUNK_LENGTH,
};
use crate::voxel::storage::VoxelMap;

fn gen_terrain(
    mut chunk_data: ResMut<VoxelMap<Voxel, ChunkShape>>,
    mut gen_queue: Local<Vec<ChunkKey>>,
    mut dirty_chunks: ResMut<DirtyChunks>,
    task_pool: Res<ComputeTaskPool>,
    gen_budget: Res<WorldTerrainGenFrameBudget>,
    chunks: Query<&Chunk, Added<Chunk>>,
) {
    gen_queue.extend(chunks.iter().map(|chunk| chunk.0));

    let drain_size = if gen_queue.len() < gen_budget.gen_per_frame {
        gen_queue.len()
    } else {
        gen_budget.gen_per_frame
    };

    //do the terrain gen here
    let generated_terrain = task_pool.scope(|scope| {
        gen_queue
            .drain(..drain_size)
            .filter_map(|key| {
                chunk_data
                    .remove(&key)
                    .and_then(|chunk_data| Some((key, chunk_data)))
            })
            .map(|(chunk_pos, mut buffer)| {
                scope.spawn_local(async move {
                    for x in (0..CHUNK_LENGTH).step_by(31) {
                        for z in 0..CHUNK_LENGTH {
                            *buffer.voxel_at_mut([x, 0, z].into()) = Voxel(1);
                            *buffer.voxel_at_mut([z, 0, x].into()) = Voxel(1);
                        }
                    }
                    (chunk_pos, buffer)
                })
            })
            .collect()
    });

    for (chunk_pos, buffer) in generated_terrain {
        chunk_data.insert(chunk_pos, buffer);
        dirty_chunks.mark_dirty(chunk_pos);
    }
}

/// Handles terrain generation.
pub struct VoxelWorldTerrainGenPlugin;

pub struct WorldTerrainGenFrameBudget {
    pub gen_per_frame: usize,
}

// we need to use a whole system stage for this in order to enable the usage of added component querries.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Hash, StageLabel)]
struct TerrainGenStage;

impl Plugin for VoxelWorldTerrainGenPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_stage_after(
            ChunkLoadingStage,
            TerrainGenStage,
            SystemStage::single(gen_terrain),
        )
        .insert_resource(WorldTerrainGenFrameBudget { gen_per_frame: 16 });
    }
}

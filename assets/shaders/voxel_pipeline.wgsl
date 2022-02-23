#import bevy_pbr::mesh_view_bind_group
#import bevy_pbr::mesh_struct

#import "shaders/voxel_data.wgsl"
#import "shaders/voxel_material.wgsl"

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] data: u32;
};

[[group(1), binding(0)]]
var<uniform> mesh: Mesh;

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] normal_pt: vec3<f32>;
    [[location(1)]] data: u32;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_position = mesh.model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = view.view_proj * world_position;
    out.normal_pt = voxel_data_extract_normal(vertex.data);
    out.data = vertex.data;

    return out;
}

struct Fragment {
    [[location(0)]] normal: vec3<f32>;
    [[location(1)]] data: u32;
};

[[stage(fragment)]]
fn fragment(frag: Fragment) -> [[location(0)]] vec4<f32> {
    let base_col = VOXEL_MATERIALS.materials[voxel_data_extract_material_index(frag.data)];
    let color = calc_voxel_lighting(base_col.xyz, frag.normal);
    return vec4<f32>(color, 1.0);
}
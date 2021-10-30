use crate::{utils, wall_constructor::Brick, CustomMesh};
use bevy::{
    prelude::*,
    render::pipeline::{PipelineDescriptor, RenderPipeline},
};
use utils::MeshBuffer;

pub struct InstancedWall {
    bevy_mesh_handle: Handle<Mesh>,
    mesh_buffer: MeshBuffer,
    entity_id: Entity,
}

impl InstancedWall {
    pub fn new(
        bricks: Vec<Brick>,
        mut mesh_assets: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        render_pipeline: Handle<PipelineDescriptor>,
        mut commands: Commands,
    ) -> Self {
        // create a mesh
        let mesh = Mesh::new(bevy::render::pipeline::PrimitiveTopology::TriangleList);
        let mesh_buffer = utils::load_gltf_as_mesh_buffer("assets/meshes/brick.glb");
        let mut out = Self {
            bevy_mesh_handle: mesh_assets.add(mesh),
            mesh_buffer,
            entity_id: Entity::new(0), // garbage, so we can init, this is overwritten right after
        };
        out.update(bricks, mesh_assets);

        out.entity_id = commands
            .spawn_bundle(PbrBundle {
                mesh: out.bevy_mesh_handle.clone(),
                material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
                //render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                //    render_pipeline,
                //)]),
                ..Default::default()
            })
            .insert(CustomMesh)
            .id();

        out
    }

    pub fn update(&mut self, bricks: Vec<Brick>, mut mesh_assets: ResMut<Assets<Mesh>>) {
        if let Some(bevy_mesh) = mesh_assets.get_mut(self.bevy_mesh_handle.clone()) {
            let mut positions: Vec<[f32; 3]> = Vec::new();
            let mut normals: Vec<[f32; 3]> = Vec::new();
            let mut uvs: Vec<[f32; 2]> = Vec::new();
            let mut indices: Vec<u32> = Vec::new();
            let mut instance_ids: Vec<u32> = Vec::new();

            let mesh_vert_count = self.mesh_buffer.indices.len();

            for (i, brick) in bricks.iter().enumerate() {
                let from_os_to_ws = Mat4::from_scale_rotation_translation(
                    brick.scale,
                    brick.rotation,
                    brick.position,
                );

                // Record WS positions
                positions.extend(
                    &self
                        .mesh_buffer
                        .positions
                        .clone()
                        .iter()
                        .map(|p| {
                            let v = from_os_to_ws.transform_point3(Vec3::from_slice_unaligned(p));
                            [v.x, v.y, v.z]
                        })
                        .collect::<Vec<_>>(),
                );

                // Record WS normals
                // Technically scaling can affect the normals, unless uniform (TODO: take non-uniform scaling into account)
                normals.extend(
                    &self
                        .mesh_buffer
                        .normals
                        .clone()
                        .iter()
                        .map(|n| {
                            let v = brick.rotation.mul_vec3(Vec3::from_slice_unaligned(n));
                            [v.x, v.y, v.z]
                        })
                        .collect::<Vec<_>>(),
                );

                uvs.extend(&self.mesh_buffer.tex_coord);

                indices.extend(
                    &self
                        .mesh_buffer
                        .indices
                        .clone()
                        .iter()
                        .map(|ind| ind + ((mesh_vert_count * i) as u32))
                        .collect::<Vec<_>>(),
                );

                instance_ids.extend(&vec![i as u32; mesh_vert_count]);
            }

            // populate bevy mesh
            bevy_mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            bevy_mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            bevy_mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
            bevy_mesh.set_attribute("Instance_Id", instance_ids);
            bevy_mesh.set_indices(Some(bevy::render::mesh::Indices::U32(indices)));
        }
    }
}

/*

pub struct Brick {
    pub pivot_u: f32,
    pub scale: Vec3,
    pub position: Vec3,
    pub rotation: Quat,
}

*/
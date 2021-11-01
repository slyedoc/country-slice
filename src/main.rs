mod curve;
mod curve_manager;
mod instanced_wall;
mod shadow_decal;
mod utils;
mod wall_constructor;

use bevy::{
    prelude::*,
    render::{
        mesh::shape,
        pipeline::{
            CompareFunction, DepthBiasState, DepthStencilState, PipelineDescriptor, RenderPipeline,
            StencilFaceState, StencilState,
        },
        render_graph::{base, RenderGraph, RenderResourcesNode},
        shader::ShaderStages,
    },
};

use bevy_dolly::prelude::*;
use bevy_mod_picking::{PickableBundle, PickingCamera, PickingCameraBundle, PickingPlugin};

use bevy::render::{
    pipeline::{
        BlendComponent, BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrite,
    },
    shader::Shader,
    texture::TextureFormat,
};

use curve::Curve;
use curve_manager::{CurveManager, UserDrawnCurve};
use wall_constructor::WallConstructor;

use bevy::{reflect::TypeUuid, render::renderer::RenderResources};
use instanced_wall::InstancedWall;
use shadow_decal::ShadowDecal;

#[derive(RenderResources, Default, TypeUuid, Component)]
#[uuid = "93fb26fc-6c05-489b-9029-601edf703b6b"]
pub struct TimeUniform {
    pub value: f32,
}

const CURVE_SHOW_DEBUG: bool = false;

// Give camera a component so we can find it and update with Dolly rig

#[derive(Component)]
struct MainCamera;

// Mark the cube that is the preview of mouse raycast intersection
#[derive(Component)]
struct PreviewCube;

#[derive(Component)]
struct CustomMesh;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(DollyPlugin)
        .add_plugin(PickingPlugin)
        .insert_resource(CurveManager::new())
        .add_startup_system(setup)
        .add_system(mouse_preview)
        .add_system(update_curve_manager.label("curve manager"))
        .add_system(update_wall_2.after("curve manager").label("wall"))
        //.add_system(handle_mouse_clicks.system())
        //.add_system(animate_shader.system()) //.after("wall"))
        .run();
}

/*
/// In this system we query for the `TimeComponent` and global `Time` resource, and set
/// `time.seconds_since_startup()` as the `value` of the `TimeComponent`. This value will be
/// accessed by the fragment shader and used to animate the shader.
fn animate_shader(time: Res<Time>, mut query: Query<&mut TimeUniform>) {
    for mut time_uniform in query.iter_mut() {
        time_uniform.value = time.seconds_since_startup() as f32;
    }
}
*/

fn update_wall_2(
    mut commands: Commands,
    mut curve_manager: ResMut<CurveManager>,
    materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let user_curves_count = curve_manager.user_curves.len();
    let wall_pipeline_handle = curve_manager.wall_pipeline_handle.clone().unwrap();
    let shadow_pipeline_handle = curve_manager.shadow_pipeline_handle.clone().unwrap();
    // If there is a curve being drawn
    if let Some(curve) = curve_manager.user_curves.last() {
        if curve.points.len() < 2 {
            return;
        }

        // Calculate brick transforms
        let curve = Curve::from(utils::smooth_points(&curve.points, 50));
        let bricks = WallConstructor::from_curve(&curve);

        // Check if there is already shadow constructed
        if let Some(shadow) = curve_manager.shadow_decals.get_mut(user_curves_count - 1) {
            shadow.update(&curve, &mut meshes);
        } else {
            curve_manager.shadow_decals.push(ShadowDecal::new(
                &curve,
                &mut meshes,
                shadow_pipeline_handle,
                &mut commands,
            ));
        }

        // Check if there is already wall constructed
        if let Some(wall) = curve_manager.instanced_walls.get_mut(user_curves_count - 1) {
            wall.update(bricks, meshes);
        } else {
            curve_manager.instanced_walls.push(InstancedWall::new(
                bricks,
                meshes,
                materials,
                wall_pipeline_handle,
                commands,
            ));
        }
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut curve_manager: ResMut<CurveManager>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    asset_server: Res<AssetServer>,
    mut render_graph: ResMut<RenderGraph>,
) {
    // Watch for changes
    asset_server.watch_for_changes().unwrap();

    curve_manager.curve_pipeline_handle = Some(pipelines.add(PipelineDescriptor::default_config(
        ShaderStages {
            vertex: asset_server.load::<Shader, _>("shaders/curve_test.vert"),
            fragment: Some(asset_server.load::<Shader, _>("shaders/curve_test.frag")),
        },
    )));

    curve_manager.wall_pipeline_handle = Some(pipelines.add(PipelineDescriptor::default_config(
        ShaderStages {
            vertex: asset_server.load::<Shader, _>("shaders/pbr.vert"),
            fragment: Some(asset_server.load::<Shader, _>("shaders/pbr.frag")),
        },
    )));

    // Same as in `build_pbr_pipeline` but with depth_write_enabled=false, because shadows are transparent
    let shadow_pipeline_descriptor = PipelineDescriptor {
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: CompareFunction::Less,
            stencil: StencilState {
                front: StencilFaceState::IGNORE,
                back: StencilFaceState::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
            bias: DepthBiasState {
                constant: 0,
                slope_scale: 0.0,
                clamp: 0.0,
            },
            //clamp_depth: false,
        }),
        color_target_states: vec![ColorTargetState {
            format: TextureFormat::default(),
            blend: Some(BlendState {
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                color: BlendComponent {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
            }),
            write_mask: ColorWrite::ALL,
        }],
        ..PipelineDescriptor::new(ShaderStages {
            vertex: asset_server.load::<Shader, _>("shaders/shadow.vert"),
            fragment: Some(asset_server.load::<Shader, _>("shaders/shadow.frag")),
        })
    };

    curve_manager.shadow_pipeline_handle = Some(pipelines.add(shadow_pipeline_descriptor));

    // Create a new shader pipeline
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
        vertex: asset_server.load::<Shader, _>("shaders/vertex_color.vert"),
        fragment: Some(asset_server.load::<Shader, _>("shaders/vertex_color.frag")),
    }));

    /*
    commands.spawn_bundle(LightBundle {
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        light: Light {
            color: Color::rgb(1.0, 0.0, 0.0),
            ..Default::default()
        },
        global_transform: Default::default(),
    });
    */

    // Add a `RenderResourcesNode` to our `RenderGraph`. This will bind `TimeComponent` to our
    // shader.
    render_graph.add_system_node(
        "time_uniform",
        RenderResourcesNode::<TimeUniform>::new(true),
    );

    // Add a `RenderGraph` edge connecting our new "time_component" node to the main pass node. This
    // ensures that "time_component" runs before the main pass.
    render_graph
        .add_node_edge("time_uniform", base::node::MAIN_PASS)
        .unwrap();

    // floor
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(utils::load_gltf_as_bevy_mesh_w_vertex_color(
                "assets/meshes/floor.glb",
            )),
            material: materials.add(Color::WHITE.into()),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                pipeline_handle,
            )]),
            ..Default::default()
        })
        .insert_bundle(PickableBundle::default());

    // preview cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(StandardMaterial {
            ..Default::default()
        }),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..Default::default()
    });

    // light
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            color: Color::rgb(1.0, 1.0, 1.0),
            intensity: 200.0,
            range: 20.0,
            radius: 0.0,
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });
    // camera
    // TODO: can replace this with a resource and update camera
    commands
        .spawn_bundle(DollyControlCameraBundle {
            rig: Rig::default(),
            transform: Transform::from_xyz(-2.0, 10.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        })
        .insert(MainCamera)
        .insert_bundle(PickingCameraBundle::default());
}

/*
fn handle_mouse_clicks(mouse_input: Res<Input<MouseButton>>, windows: Res<Windows>) {
    let win = windows.get_primary().expect("no primary window");
    if mouse_input.just_pressed(MouseButton::Left) {
        //println!("click at {:?}", win.cursor_position());
    }
}
*/

fn mouse_preview(
    mut query: Query<&mut PickingCamera>,
    mut cube_query: Query<(&mut PreviewCube, &mut Transform)>,
) {
    for camera in query.iter_mut() {
        if let Some((_, intersection)) = camera.intersect_top() {
            for (_, mut transform) in cube_query.iter_mut() {
                transform.translation = intersection.position();
            }
        }
    }
}

fn update_curve_manager(
    materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    mut curve_manager: ResMut<CurveManager>,
    mouse_button_input: Res<Input<MouseButton>>,
    keys: Res<Input<KeyCode>>,
    mut query: Query<&mut PickingCamera>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        curve_manager.clear_all(&mut commands);
    }

    // If LMB was just pressed, start a new curve
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let pipeline = curve_manager.curve_pipeline_handle.clone().unwrap();
        curve_manager
            .user_curves
            .push(UserDrawnCurve::new(pipeline));
    }

    // If there is a curve being drawn
    if let Some(curve) = curve_manager.user_curves.last_mut() {
        // Add points to it
        if mouse_button_input.pressed(MouseButton::Left) {
            let camera = query.single_mut();
            if let Some((_, intersection)) = camera.intersect_top() {
                const DIST_THRESHOLD: f32 = 0.001;

                if curve
                    .points
                    .last()
                    // if curve  had points, only add if the distance is larger than X
                    .map(|last| intersection.position().distance(*last) > DIST_THRESHOLD)
                    // if curve  has no points, add this point
                    .unwrap_or(true)
                {
                    curve.points.push(intersection.position())
                }
            }
        }

        // Update its debug mesh
        if CURVE_SHOW_DEBUG {
            curve.update_debug_mesh(meshes, materials, commands);
        }
    }
}

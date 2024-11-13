//! A Node Graph Editor for Bevy.

use bevy::{
    color::palettes::css::RED,
    picking::{
        pointer::{Location, PointerId, PointerInput, PointerLocation},
        PickSet,
    },
    prelude::*,
    render::{
        camera::{NormalizedRenderTarget, RenderTarget},
        render_resource::{Extent3d, TextureFormat, TextureUsages},
        view::RenderLayers,
    },
    ui::ui_layout_system,
};
use bevy_editor_camera::{EditorCamera2d, EditorCamera2dPlugin};
use bevy_editor_styles::Theme;
use bevy_infinite_grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings};
use bevy_pane_layout::{PaneContentNode, PaneRegistry};

/// The identifier for the 2D Viewport.
/// This is present on any pane that is a Node Graph.
#[derive(Component)]
struct NodeGraph {
    camera: Entity,
}

impl Default for NodeGraph {
    fn default() -> Self {
        NodeGraph {
            camera: Entity::PLACEHOLDER,
        }
    }
}

/// Plugin for the Node graph pane.
pub struct NodeGraphPlugin;

impl Plugin for NodeGraphPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<InfiniteGridPlugin>() {
            app.add_plugins(InfiniteGridPlugin);
        }
        if !app.is_plugin_added::<EditorCamera2dPlugin>() {
            app.add_plugins(EditorCamera2dPlugin);
        }
        app.add_systems(Startup, setup)
            .add_systems(
                PreUpdate,
                render_target_picking_passthrough.in_set(PickSet::Last),
            )
            .add_systems(
                PostUpdate,
                update_render_target_size.after(ui_layout_system),
            )
            .add_observer(on_pane_creation)
            .add_observer(
                |trigger: Trigger<OnRemove, NodeGraph>,
                 mut commands: Commands,
                 query: Query<&NodeGraph>| {
                    // Despawn the viewport camera
                    commands
                        .entity(query.get(trigger.entity()).unwrap().camera)
                        .despawn_recursive();
                },
            );

        app.world_mut()
            .get_resource_or_init::<PaneRegistry>()
            .register("Node Graph", |mut commands, pane_root| {
                commands.entity(pane_root).insert(NodeGraph::default());
            });
    }
}

#[derive(Component)]
struct Active;

fn render_target_picking_passthrough(
    mut commands: Commands,
    viewports: Query<(Entity, &NodeGraph)>,
    content: Query<&PaneContentNode>,
    children_query: Query<&Children>,
    node_query: Query<(&ComputedNode, &GlobalTransform, &UiImage), With<Active>>,
    mut pointers: Query<(&PointerId, &mut PointerLocation)>,
    mut pointer_input_reader: EventReader<PointerInput>,
) {
    for event in pointer_input_reader.read() {
        // Ignore the events we send to the render-targets
        if !matches!(event.location.target, NormalizedRenderTarget::Window(..)) {
            continue;
        }
        for (pane_root, _viewport) in &viewports {
            let content_node_id = children_query
                .iter_descendants(pane_root)
                .find(|e| content.contains(*e))
                .unwrap();

            let image_id = children_query.get(content_node_id).unwrap()[0];

            let Ok((computed_node, global_transform, ui_image)) = node_query.get(image_id) else {
                // Inactive viewport
                continue;
            };
            let node_rect =
                Rect::from_center_size(global_transform.translation().xy(), computed_node.size());

            let new_location = Location {
                position: event.location.position - node_rect.min,
                target: NormalizedRenderTarget::Image(ui_image.texture.clone()),
            };

            // Duplicate the event
            let mut new_event = event.clone();
            // Relocate the event to the render-target
            new_event.location = new_location.clone();
            // Resend the event
            commands.send_event(new_event);

            if let Some((_id, mut pointer_location)) = pointers
                .iter_mut()
                .find(|(pointer_id, _)| **pointer_id == event.pointer_id)
            {
                // Relocate the pointer to the render-target
                pointer_location.location = Some(new_location);
            }
        }
    }
}

fn setup(mut commands: Commands, theme: Res<Theme>) {
    commands.spawn((
        InfiniteGrid,
        InfiniteGridSettings {
            scale: 100.,
            dot_fadeout_strength: 0.,
            x_axis_color: theme.viewport.x_axis_color,
            z_axis_color: theme.viewport.y_axis_color,
            major_line_color: theme.viewport.grid_major_line_color,
            minor_line_color: theme.viewport.grid_minor_line_color,
            ..default()
        },
        Transform::from_rotation(Quat::from_rotation_arc(Vec3::Y, Vec3::Z)),
        RenderLayers::layer(11),
    ));
}

#[allow(clippy::too_many_arguments)]
fn on_pane_creation(
    trigger: Trigger<OnAdd, NodeGraph>,
    mut commands: Commands,
    children_query: Query<&Children>,
    mut query: Query<&mut NodeGraph>,
    content: Query<&PaneContentNode>,
    mut images: ResMut<Assets<Image>>,
    theme: Res<Theme>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let pane_root = trigger.entity();
    let content_node = children_query
        .iter_descendants(pane_root)
        .find(|e| content.contains(*e))
        .unwrap();

    let mut image = Image::default();

    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    image.texture_descriptor.format = TextureFormat::Bgra8UnormSrgb;

    let image_handle = images.add(image);

    let image_id = commands
        .spawn((
            UiImage {
                texture: image_handle.clone(),
                ..Default::default()
            },
            Node {
                position_type: PositionType::Absolute,
                top: Val::ZERO,
                bottom: Val::ZERO,
                left: Val::ZERO,
                right: Val::ZERO,
                ..default()
            },
        ))
        .observe(|trigger: Trigger<Pointer<Over>>, mut commands: Commands| {
            commands.entity(trigger.entity()).insert(Active);
        })
        .observe(|trigger: Trigger<Pointer<Out>>, mut commands: Commands| {
            commands.entity(trigger.entity()).remove::<Active>();
        })
        .set_parent(content_node)
        .id();

    let camera_id = commands
        .spawn((
            Camera2d,
            EditorCamera2d {
                enabled: false,
                ..default()
            },
            Camera {
                target: RenderTarget::Image(image_handle),
                clear_color: ClearColorConfig::Custom(theme.viewport.background_color),
                ..default()
            },
            RenderLayers::layer(11),
        ))
        .id();

    commands
        .entity(image_id)
        .observe(
            move |_trigger: Trigger<Pointer<Move>>, mut query: Query<&mut EditorCamera2d>| {
                let mut editor_camera = query.get_mut(camera_id).unwrap();
                editor_camera.enabled = true;
            },
        )
        .observe(
            move |_trigger: Trigger<Pointer<Out>>, mut query: Query<&mut EditorCamera2d>| {
                query.get_mut(camera_id).unwrap().enabled = false;
            },
        );

    query.get_mut(pane_root).unwrap().camera = camera_id;
}

fn update_render_target_size(
    query: Query<(Entity, &NodeGraph)>,
    mut camera_query: Query<(&Camera, &mut EditorCamera2d)>,
    content: Query<&PaneContentNode>,
    children_query: Query<&Children>,
    pos_query: Query<
        (&ComputedNode, &GlobalTransform),
        Or<(Changed<ComputedNode>, Changed<GlobalTransform>)>,
    >,
    mut images: ResMut<Assets<Image>>,
) {
    for (pane_root, viewport) in &query {
        let content_node_id = children_query
            .iter_descendants(pane_root)
            .find(|e| content.contains(*e))
            .unwrap();

        let Ok((computed_node, global_transform)) = pos_query.get(content_node_id) else {
            continue;
        };
        // TODO Convert to physical pixels
        let content_node_size = computed_node.size();

        let node_position = global_transform.translation().xy();
        let rect = Rect::from_center_size(node_position, computed_node.size());

        let (camera, mut editor_camera) = camera_query.get_mut(viewport.camera).unwrap();

        editor_camera.viewport_override = Some(rect);

        let image_handle = camera.target.as_image().unwrap();
        let size = Extent3d {
            width: u32::max(1, content_node_size.x as u32),
            height: u32::max(1, content_node_size.y as u32),
            depth_or_array_layers: 1,
        };
        images.get_mut(image_handle).unwrap().resize(size);
    }
}

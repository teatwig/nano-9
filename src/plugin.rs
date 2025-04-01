#![allow(deprecated)]
use bevy::{
    image::ImageSampler,
    prelude::*,
    reflect::Reflect,
    render::{
        camera::ScalingMode,
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    utils::Duration,
    window::{PresentMode, PrimaryWindow, WindowMode, WindowResized},
};

use bevy_mod_scripting::{
    core::{
        asset::ScriptAsset,
        bindings::{function::namespace::NamespaceBuilder, script_value::ScriptValue},
        callback_labels,
        event::ScriptCallbackEvent,
        handler::event_handler,
    },
    lua::LuaScriptingPlugin,
};

use crate::{config::*, error::ErrorState, pico8::fill_input, N9Var};

#[derive(Component)]
pub struct Nano9Sprite;

#[derive(Resource)]
pub struct Nano9Screen(pub Handle<Image>);

#[derive(Clone, Debug)]
pub struct DrawState {
    pub pen: Color,
    pub camera_position: Vec2,
    pub print_cursor: Vec2,
}

#[derive(Debug, Clone, Resource, Default)]
pub struct N9Canvas {
    pub size: UVec2,
    pub handle: Handle<Image>,
}

impl Default for DrawState {
    fn default() -> Self {
        DrawState {
            pen: Srgba::rgb(0.761, 0.765, 0.780).into(), // color 6, palette
            camera_position: Vec2::ZERO,
            print_cursor: Vec2::ZERO,
        }
    }
}

pub fn setup_canvas(mut canvas: Option<ResMut<N9Canvas>>, mut assets: ResMut<Assets<Image>>) {
    if let Some(ref mut canvas) = canvas {
        let mut image = Image::new_fill(
            Extent3d {
                width: canvas.size.x,
                height: canvas.size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0u8, 0u8, 0u8, 0u8],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        canvas.handle = assets.add(image);
    }
}

pub mod call {
    use super::*;
    callback_labels!(
    SetGlobal => "_set_global",
    Update => "_update",
    Update60 => "_update60",
    Init => "_init",
    Eval => "_eval",
    Draw => "_draw"); // TODO: Should permit trailing comma
}

// pub fn set_camera(
//     camera: Query<Entity, With<Camera>>,
//     mut events: PriorityEventWriter<LuaEvent<N9Args>>,
// ) {
//     if let Ok(id) = camera.get_single() {
//         events.send(
//             LuaEvent {
//                 hook_name: "_set_global".to_owned(),
//                 args: {
//                     let mut args = Variadic::new();
//                     args.push(N9Arg::String("camera".into()));
//                     // args.push(N9Arg::Entity(id));
//                     args.push(N9Arg::Camera(N9Camera(id)));
//                     args
//                 },
//                 recipients: Recipients::All,
//             },
//             0,
//         )
//     }
// }

#[derive(Component, Debug, Reflect)]
pub struct Nano9Camera;

fn spawn_camera(mut commands: Commands, canvas: Option<Res<N9Canvas>>) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::WindowSize;
    let handle = canvas.as_ref().map(|c| c.handle.clone());
    let canvas_size: UVec2 = canvas.map(|c| c.size).unwrap_or_default();
    commands
        .spawn((
            Transform::from_xyz(
                canvas_size.x as f32 / 2.0,
                -(canvas_size.y as f32) / 2.0,
                0.0,
            ),
            InheritedVisibility::default(),
            Name::new("dolly"),
        ))
        .with_children(|parent| {
            let mut camera_commands = parent.spawn((
                Camera2d,
                Msaa::Off,
                // Projection::from(projection),
                projection,
                IsDefaultUiCamera,
                InheritedVisibility::default(),
                Nano9Camera,
                N9Var::new("camera"),
                Name::new("camera"),
            ));
            if let Some(handle) = handle {
                camera_commands.with_children(|parent| {
                    parent.spawn((
                        Sprite::from_image(handle),
                        // transform: Transform::from_xyz(64.0, 64.0, -1.0),
                        Transform::from_xyz(0.0, 0.0, -100.0),
                        //.with_scale(Vec3::splat(settings.pixel_scale)),
                        Nano9Sprite,
                        N9Var::new("canvas"),
                        Name::new("canvas"),
                    ));
                });
            }
        });
}

pub fn fullscreen_key(
    input: Res<ButtonInput<KeyCode>>,
    mut primary_windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if input.just_pressed(KeyCode::Enter)
        && input.any_pressed([KeyCode::AltLeft, KeyCode::AltRight])
    {
        use WindowMode::*;
        let mut primary_window = primary_windows.single_mut();
        primary_window.mode = match primary_window.mode {
            Windowed => Fullscreen(MonitorSelection::Current),
            _ => Windowed,
        }
    }
}

pub fn sync_window_size(
    mut resize_event: EventReader<WindowResized>,
    canvas: Res<N9Canvas>,
    // mut query: Query<&mut Sprite, With<Nano9Sprite>>,
    primary_windows: Query<&Window, With<PrimaryWindow>>,
    mut orthographic: Single<&mut OrthographicProjection, With<Nano9Camera>>,
) {
    if let Some(e) = resize_event
        .read()
        .filter(|e| primary_windows.get(e.window).is_ok())
        .last()
    {
        let primary_window = primary_windows.get(e.window).unwrap();

        //let window_size = primary_window.physical_size().as_vec2();
        let window_scale = primary_window.scale_factor();
        let window_size = Vec2::new(
            primary_window.physical_width() as f32,
            primary_window.physical_height() as f32,
        ) / window_scale;
        // let mut orthographic = orthographic.single_mut();

        let canvas_size = canvas.size.as_vec2();
        // let canvas_aspect = canvas_size.x / canvas_size.y;
        // let window_aspect = window_size.x / window_size.y;

        let new_scale =
                // Canvas is longer than it is tall. Fit the width first.
                (window_size.y / canvas_size.y).min(window_size.x / canvas_size.x);
        // info!("window_size {window_size}");

        // match *orthographic.into_inner() {
        //     Projection::Orthographic(ref mut orthographic) => {

        // info!("oldscale {} new_scale {new_scale}", &orthographic.scale);
        orthographic.scale = 1.0 / new_scale;
        // }
        //     _ => { panic!("Not expecting a perspective"); }

        // }
        // settings.pixel_scale = new_scale;
        // orthographic.scaling_mode = ScalingMode::WindowSize;
        // }
        // transform.scale = Vec3::splat(new_scale);

        // let scale = if settings.canvas_size.x > settings.canvas_size.y
        // {
        //     // horizontal is longer
        //     settings.resolution.1 as f32
        //         / settings.canvas_size.y as f32
        // } else {
        //     // vertical is longer
        //     settings.resolution.0 as f32
        //         / settings.canvas_size.x as f32
        // };

        //     sprite.custom_size = Some(Vec2::new(
        //         (settings.canvas_size.x as f32) * scale,
        //         (settings.canvas_size.y as f32) * scale,
        //     ));
    }
}

/// Sends events allowing scripts to drive update logic
pub fn send_update(
    mut writer: EventWriter<ScriptCallbackEvent>,
    frame_count: Res<bevy::core::FrameCount>,
) {
    writer.send(ScriptCallbackEvent::new_for_all(
        call::Update,
        vec![ScriptValue::Unit],
    ));
}

pub fn send_update60(mut writer: EventWriter<ScriptCallbackEvent>) {
    writer.send(ScriptCallbackEvent::new_for_all(
        call::Update60,
        vec![ScriptValue::Unit],
    ));
}

/// Sends initialization event
pub fn send_init(
    mut writer: EventWriter<ScriptCallbackEvent>,
    // mut loaded: EventReader<OnScriptLoaded>,
) {
    info!("calling init");
    // todo!("PUT INIT ELSEWHERE like Lua's on_script_loaded()");
    // for e in loaded.read() {
    // eprintln!("init {}", e.sid);
    writer.send(ScriptCallbackEvent::new_for_all(
        call::Init,
        vec![ScriptValue::Unit],
    ));
}

/// Sends draw event
pub fn send_draw(mut writer: EventWriter<ScriptCallbackEvent>) {
    writer.send(ScriptCallbackEvent::new_for_all(
        call::Draw,
        vec![ScriptValue::Unit],
    ));
}
const DEFAULT_FRAMES_PER_SECOND: u8 = 60;

#[derive(Default)]
pub struct Nano9Plugin {
    pub config: Config,
}

impl Nano9Plugin {
    pub fn window_plugin(&self) -> WindowPlugin {
        let screen_size = self
            .config
            .screen
            .as_ref()
            .and_then(|s| s.screen_size)
            .unwrap_or(DEFAULT_SCREEN_SIZE);
        WindowPlugin {
            primary_window: Some(Window {
                resolution: screen_size.as_vec2().into(), //WindowResolution::new(resolution.x, resolution.y),
                title: self
                    .config
                    .name
                    .as_deref()
                    .unwrap_or_else(|| "Nano-9")
                    .into(),
                // Turn off vsync to maximize CPU/GPU usage
                present_mode: PresentMode::AutoVsync,
                // Let's not allow resizing.
                // resize_constraints: WindowResizeConstraints {
                //     min_width: resolution.x,
                //     max_width: resolution.x,
                //     min_height: resolution.y,
                //     max_height: resolution.y,
                // },
                ..default()
            }),
            ..default()
        }
    }
}

fn add_info(app: &mut App) {
    let world = app.world_mut();
    NamespaceBuilder::<World>::new_unregistered(world)
        .register("info", |s: String| {
            bevy::log::info!(s);
        })
        .register("warn", |s: String| {
            bevy::log::warn!(s);
        })
        .register("error", |s: String| {
            bevy::log::error!(s);
        })
        .register("debug", |s: String| {
            bevy::log::debug!(s);
        });
}

impl Plugin for Nano9Plugin {
    fn build(&self, app: &mut App) {
        // How do you enable shared context since it eats the plugin?
        let mut lua_scripting_plugin = LuaScriptingPlugin::default();
        let canvas_size: UVec2 = self
            .config
            .screen
            .as_ref()
            .map(|s| s.canvas_size)
            .unwrap_or(DEFAULT_CANVAS_SIZE);
        lua_scripting_plugin
            .scripting_plugin
            .add_context_initializer(
                |_script_id: &str, context: &mut bevy_mod_scripting::lua::mlua::Lua| {
                    context.globals().set(
                        "_eval_string",
                        context.create_function(|ctx, arg: String| {
                            Ok(ctx.load(format!("tostring({arg})")).eval::<String>()?)
                        })?,
                    )?;

                    context
                        .load(include_str!("builtin.lua"))
                        .exec()
                        .expect("Problem in builtin.lua");
                    Ok(())
                },
            );
        // let resolution = settings.canvas_size.as_vec2() * settings.pixel_scale;
        app.insert_resource(bevy::winit::WinitSettings {
            // focused_mode: bevy::winit::UpdateMode::Continuous,
            focused_mode: bevy::winit::UpdateMode::reactive(Duration::from_millis(16)),
            unfocused_mode: bevy::winit::UpdateMode::reactive_low_power(Duration::from_millis(
                16 * 4,
            )),
        })
        .insert_resource(N9Canvas {
            size: canvas_size,
            ..default()
        })
        // Insert the config as a resource.
        // TODO: Should we constrain it, if it wasn't provided as an option?
        .insert_resource(Time::<Fixed>::from_seconds(
            1.0 / self
                .config
                .frames_per_second
                .unwrap_or(DEFAULT_FRAMES_PER_SECOND) as f64,
        ))
        .add_plugins((lua_scripting_plugin, crate::plugin, add_info))
        .add_systems(Startup, (setup_canvas, spawn_camera).chain())
        .add_systems(
            Update,
            (
                fill_input,
                send_init.run_if(on_asset_change::<ScriptAsset>()),
                event_handler::<call::Init, LuaScriptingPlugin>,
                send_update.run_if(in_state(ErrorState::None)),
                event_handler::<call::Update, LuaScriptingPlugin>,
                event_handler::<call::Eval, LuaScriptingPlugin>,
                send_draw.run_if(in_state(ErrorState::None)),
                event_handler::<call::Draw, LuaScriptingPlugin>,
            )
                .chain(),
        );

        // bevy_ecs_ldtk will add this plugin, so let's not add that if it's present.
        #[cfg(not(feature = "level"))]
        app.add_plugins(bevy_ecs_tilemap::TilemapPlugin);

        if app.is_plugin_added::<WindowPlugin>() {
            app.add_systems(Update, sync_window_size)
                .add_systems(Update, fullscreen_key);
        }
    }
}

pub fn on_asset_change<T: Asset>() -> impl FnMut(EventReader<AssetEvent<T>>) -> bool + Clone {
    // The events need to be consumed, so that there are no false positives on subsequent
    // calls of the run condition. Simply checking `is_empty` would not be enough.
    // PERF: note that `count` is efficient (not actually looping/iterating),
    // due to Bevy having a specialized implementation for events.
    move |mut reader: EventReader<AssetEvent<T>>| {
        reader
            .read()
            .inspect(|e| info!("asset event {e:?}"))
            .any(|e| {
                matches!(
                    e, //AssetEvent::LoadedWithDependencies { .. } |
                    AssetEvent::Added { .. } | AssetEvent::Modified { .. }
                )
            })
    }
}

pub fn on_asset_modified<T: Asset>() -> impl FnMut(EventReader<AssetEvent<T>>) -> bool + Clone {
    // The events need to be consumed, so that there are no false positives on subsequent
    // calls of the run condition. Simply checking `is_empty` would not be enough.
    // PERF: note that `count` is efficient (not actually looping/iterating),
    // due to Bevy having a specialized implementation for events.
    move |mut reader: EventReader<AssetEvent<T>>| {
        reader
            .read()
            .any(|e| matches!(e, AssetEvent::Modified { .. }))
    }
}

pub fn info_on_asset_event<T: Asset>() -> impl FnMut(EventReader<AssetEvent<T>>) {
    // The events need to be consumed, so that there are no false positives on subsequent
    // calls of the run condition. Simply checking `is_empty` would not be enough.
    // PERF: note that `count` is efficient (not actually looping/iterating),
    // due to Bevy having a specialized implementation for events.
    move |mut reader: EventReader<AssetEvent<T>>| {
        for event in reader.read() {
            match event {
                AssetEvent::Modified { .. } => (),
                _ => {
                    info!("ASSET EVENT {:?}", &event);
                }
            }
        }
    }
}

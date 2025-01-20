#![allow(deprecated)]
use bevy::{
    audio::AudioPlugin,
    ecs::prelude::Condition,
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
use std::sync::{Arc, Mutex};

use bevy_mod_scripting::{core::{callback_labels, bindings::script_value::ScriptValue, event::{ScriptCallbackEvent, OnScriptLoaded}}};

use crate::{
    error::ErrorState,
    screens, DropPolicy, //N9Camera, N9Sprite,
    N9Var, N9Entity,
};

#[derive(Component)]
pub struct Nano9Sprite;

#[derive(Resource)]
pub struct Nano9SpriteSheet(pub Handle<Image>, pub Handle<TextureAtlasLayout>);

#[derive(Resource)]
pub struct Nano9Screen(pub Handle<Image>);

#[derive(Resource, Clone, Debug)]
pub struct DrawState {
    pub pen: Color,
    pub camera_position: UVec2,
    pub print_cursor: UVec2,
}

impl Default for DrawState {
    fn default() -> Self {
        DrawState {
            pen: Srgba::rgb(0.761, 0.765, 0.780).into(), // color 6, palette
            camera_position: UVec2::ZERO,
            print_cursor: UVec2::ZERO,
        }
    }
}

#[derive(Reflect, Resource)]
#[reflect(Resource)]
pub struct Settings {
    pixel_dimensions: UVec2,
    resolution: Vec2,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pixel_dimensions: UVec2::splat(128),
            resolution: Vec2::splat(512.0),
        }
    }
}

pub fn setup_image(
    mut commands: Commands,
    mut assets: ResMut<Assets<Image>>,
    settings: Res<N9Settings>,
) {
    let image = Image::new_fill(
        Extent3d {
            width: settings.canvas_size.x,
            height: settings.canvas_size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8, 0u8, 0u8, 0u8],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    let handle = assets.add(image);
    commands.insert_resource(Nano9Screen(handle.clone()));
}

pub mod call {
    use super::*;
callback_labels!(
    SetGlobal => "_set_global",
    Update => "_update",
    Update60 => "_update60",
    Init => "_init",
    Draw => "_draw"); // TODO: Should permit trailing comma
}

pub fn set_background(
    screen: Query<Entity, With<Nano9Sprite>>,
    mut writer: EventWriter<ScriptCallbackEvent>,
) {
    if let Ok(id) = screen.get_single() {
        writer.send(ScriptCallbackEvent::new_for_all(
            call::SetGlobal,
            vec![ScriptValue::String("background".into()),
                 ScriptValue::Reference(Arc::new(Mutex::new(N9Entity { entity: id,
                                                          drop: DropPolicy::Nothing })))]));
        // events.send(
        //     LuaEvent {
        //         hook_name: "_set_global".to_owned(),
        //         args: {
        //             let mut args = Variadic::new();
        //             args.push(N9Arg::String("background".into()));
        //             args.push(N9Arg::Sprite(Arc::new(Mutex::new(N9Sprite {
        //                 entity: id,
        //                 drop: DropPolicy::Nothing,
        //             }))));
        //             // args.push(N9Arg::DropPolicy(DropPolicy::Nothing));
        //             // N9Arg::SetSprite {
        //             // name: "background".into(),
        //             // sprite: id,
        //             // drop: DropPolicy::Nothing,
        //             args
        //         },
        //         recipients: Recipients::All,
        //     },
        //     0,
        // );
    }
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

fn spawn_camera(mut commands: Commands, settings: Res<N9Settings>, screen: Res<Nano9Screen>) {
    let mut projection = OrthographicProjection::default_2d();
    // camera_bundle.transform = Transform::from_xyz(64.0, 64.0, 0.0);
    // camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(512.0);
    projection.scaling_mode = ScalingMode::WindowSize; //(settings.pixel_scale);
    projection.scale = 1.0 / settings.pixel_scale;


    commands
        .spawn((
            Camera2d,
            Transform::from_xyz(64.0, -64.0, 0.0),//.looking_to(Dir3::Z, Dir3::NEG_Y),
            Projection::from(projection),
            IsDefaultUiCamera,
            InheritedVisibility::default(),
            N9Var::new("camera"),
            Name::new("camera"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Sprite::from_image(screen.0.clone()),
                // transform: Transform::from_xyz(64.0, 64.0, -1.0),
                Transform::from_xyz(0.0, 0.0, -100.0),
                //.with_scale(Vec3::splat(settings.pixel_scale)),
                Nano9Sprite,
                N9Var::new("background"),
                Name::new("background"),
            ));
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
    mut settings: ResMut<N9Settings>,
    // mut query: Query<&mut Sprite, With<Nano9Sprite>>,
    primary_windows: Query<&Window, With<PrimaryWindow>>,
    mut orthographic: Query<&mut OrthographicProjection, With<Camera>>,
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
        let mut orthographic = orthographic.single_mut();

        let canvas_size = settings.canvas_size.as_vec2();
        // let canvas_aspect = canvas_size.x / canvas_size.y;
        // let window_aspect = window_size.x / window_size.y;

        let new_scale =
            // Canvas is longer than it is tall. Fit the width first.
            (window_size.y / canvas_size.y).min(window_size.x / canvas_size.x);
        // info!("window_size {window_size}");
        // info!("new_scale {new_scale}");
        settings.pixel_scale = new_scale;
        // orthographic.scaling_mode = ScalingMode::WindowSize(new_scale);
        orthographic.scale = 1.0 / new_scale;
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
                   frame_count: Res<bevy::core::FrameCount>) {
    if frame_count.0 % 2 == 0 {

        writer.send(ScriptCallbackEvent::new_for_all(
            call::Update,
            vec![ScriptValue::Unit]));
    // events.send(
    //     LuaEvent {
    //         hook_name: "_update".to_owned(),
    //         args: N9Args::new(),
    //         recipients: Recipients::All,
    //     },
    //     1,
    // )
    }
}

pub fn send_update60(
    mut writer: EventWriter<ScriptCallbackEvent>,
) {
        writer.send(ScriptCallbackEvent::new_for_all(
            call::Update60,
            vec![ScriptValue::Unit]));
}

/// Sends initialization event
pub fn send_init(
    mut writer: EventWriter<ScriptCallbackEvent>,
    // mut loaded: EventReader<OnScriptLoaded>,
) {
    todo!("PUT INIT ELSEWHERE like Lua's on_script_loaded()");
    // for e in loaded.read() {
    //     eprintln!("init {}", e.sid);
    //     writer.send(ScriptCallbackEvent::new_for_all(
    //         call::Init,
    //         vec![ScriptValue::Unit]));
    //     // events.send(
    //     //     LuaEvent {
    //     //         hook_name: "_init".to_owned(),
    //     //         args: N9Args::new(),
    //     //         recipients: Recipients::ScriptID(e.sid),
    //     //     },
    //     //     0,
    //     // )
    // }
}

/// Sends draw event
pub fn send_draw(
    mut writer: EventWriter<ScriptCallbackEvent>,
) {
    writer.send(ScriptCallbackEvent::new_for_all(
        call::Draw,
        vec![ScriptValue::Unit]));
}
const UPDATE_FREQUENCY: f32 = 1.0 / 60.0;

#[derive(Default)]
pub struct Nano9Plugin {
    settings: N9Settings,
}

#[derive(Resource)]
pub struct N9Settings {
    canvas_size: UVec2,
    pixel_scale: f32,
}

impl Default for N9Settings {
    fn default() -> Self {
        Self {
            canvas_size: UVec2::splat(128),
            pixel_scale: 4.0,
        }
    }
}

impl Nano9Plugin {
    pub fn default_plugins(&self) -> bevy::app::PluginGroupBuilder {
        let settings = &self.settings;
        let resolution = settings.canvas_size.as_vec2() * settings.pixel_scale;
        DefaultPlugins
            .set(AudioPlugin {
                global_volume: GlobalVolume::new(0.4),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: resolution.into(),//WindowResolution::new(resolution.x, resolution.y),
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
            })
            .set(ImagePlugin::default_nearest())
            .build()
    }
}

impl Plugin for Nano9Plugin {
    fn build(&self, app: &mut App) {
        // let resolution = settings.canvas_size.as_vec2() * settings.pixel_scale;
        app.insert_resource(bevy::winit::WinitSettings {
            // focused_mode: bevy::winit::UpdateMode::Continuous,
            focused_mode: bevy::winit::UpdateMode::reactive(Duration::from_millis(16)),
            unfocused_mode: bevy::winit::UpdateMode::reactive_low_power(Duration::from_millis(
                16 * 4,
            )),
        })
        .insert_resource(Time::<Fixed>::from_seconds(UPDATE_FREQUENCY.into()))
        .init_resource::<N9Settings>()
        .init_resource::<DrawState>()
        .add_plugins(crate::plugin)
        .add_plugins(bevy_ecs_tilemap::TilemapPlugin)
        // .add_systems(OnExit(screens::Screen::Loading), setup_image)
        // .add_systems(Startup, (setup_image, spawn_camera, set_camera).chain())
        .add_systems(Startup, (setup_image, spawn_camera).chain())
        // .add_systems(OnEnter(screens::Screen::Playing), send_init)
        // .add_systems(PreUpdate, send_init.run_if(on_asset_modified::<LuaFile>()))
        .add_systems(PreUpdate, send_init.run_if(on_event::<OnScriptLoaded>))
        // .add_systems(
        //     PreUpdate,
        //     (set_background, set_camera)
        //         .chain()
        //         ),
        // )
        // .add_systems(PreUpdate, (send_init).chain().run_if(on_event::<OnScriptLoaded>()))
        // .add_systems(Update, info_on_asset_event::<Image>())
        .add_systems(
            Update,
            ((send_update, send_update60), send_draw)
                .chain()
                .run_if(in_state(screens::Screen::Playing)
                        .and_then(in_state(ErrorState::None))),
        );

        if app.is_plugin_added::<WindowPlugin>() {
            app.add_systems(Update, sync_window_size)
                .add_systems(Update, fullscreen_key);
        }
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

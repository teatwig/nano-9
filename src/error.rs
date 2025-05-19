use bevy::{color::palettes::css, core::FrameCount, prelude::*, window::RequestRedraw};
use bevy_mod_scripting::core::{asset::ScriptAsset, event::ScriptErrorEvent};

pub(crate) fn plugin(app: &mut App) {
    app.init_state::<RunState>()
        .add_systems(Startup, spawn_error_message_layout)
        .add_systems(Update, add_messages);

    if app.is_plugin_added::<WindowPlugin>() {
        app
            .add_systems(PreUpdate, try_to_run_after_error)
            .add_systems(OnEnter(RunState::Messages), show::<ErrorMessages>)
            .add_systems(OnExit(RunState::Messages), (clear_messages, hide::<ErrorMessages>));
    }
}

const FONT_SIZE: f32 = 24.0;
const PADDING: Val = Val::Px(5.);
const LEFT_PADDING: Val = Val::Px(10.);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States, Reflect)]
pub enum RunState {
    #[default]
    Uninit,
    Run,
    Pause,
    /// Error messages
    ///
    /// XXX: Change name to suit.
    Messages,
}

#[derive(Component)]
pub struct ErrorMessages;

/// Make component visible.
pub fn show<T: Component>(
    mut redraw: EventWriter<RequestRedraw>,
    mut query: Query<&mut Visibility, With<T>>,
) {
    if let Ok(mut visibility) = query.get_single_mut() {
        *visibility = Visibility::Visible;
        redraw.send(RequestRedraw);
    }
}

/// Make component visible.
pub fn hide<T: Component>(
    mut redraw: EventWriter<RequestRedraw>,
    mut query: Query<&mut Visibility, With<T>>,
) {
    if let Ok(mut visibility) = query.get_single_mut() {
        *visibility = Visibility::Hidden;
        redraw.send(RequestRedraw);
    }
}

fn spawn_error_message_layout(mut commands: Commands) {
    commands
        .spawn((Node {
            position_type: PositionType::Absolute,
            // top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            right: Val::Px(0.0),
            left: Val::Px(0.0),
            flex_direction: FlexDirection::Column,

            // align_items: AlignItems::FlexEnd,
            // justify_content:
            ..Default::default()
        },))
        .with_children(|parent| {
            parent.spawn((
                Visibility::Hidden,
                Node {
                    flex_direction: FlexDirection::Column,
                    // flex_grow: 1.,
                    padding: UiRect {
                        top: PADDING,
                        left: LEFT_PADDING,
                        right: PADDING,
                        bottom: PADDING,
                    },
                    ..Default::default()
                },
                BackgroundColor(css::RED.into()),
                ErrorMessages,
            ));
            // .with_children(|parent| {

            //     let error_style: TextStyle = TextStyle {
            //         font_size: FONT_SIZE,
            //         ..default()
            //     };
            //     parent.spawn((
            //         TextBundle::from_section("Test", error_style),
            //         ErrorMessages));
            // })
            // .with_background_color(Color::RED));
        });
}

pub fn add_messages(
    mut r: EventReader<ScriptErrorEvent>,
    query: Query<Entity, With<ErrorMessages>>,
    _frame_count: Res<FrameCount>,
    mut state: ResMut<NextState<RunState>>,
    mut commands: Commands,
) {
    if r.is_empty() {
        return;
    }
    let id = query.single();
    commands.entity(id).with_children(|parent| {
        for e in r.read() {
            // eprintln!("XXXX\n\n err {}", e.error);

            let error_style = TextFont::default().with_font_size(FONT_SIZE);
            // let msg = match &e.error {
            //     ScriptError::FailedToLoad { script: _, msg } => msg.clone(),
            //     x => format!("{}", &x.error),
            // };
            let msg = format!("{}", &e.error);
            parent.spawn((Text::new(msg), error_style));
        }
    });

    state.set(RunState::Messages {
        // frame: frame_count.0,
    });
}

/// Clear any error messages.
pub fn clear_messages(
    query: Query<Entity, With<ErrorMessages>>,
    mut commands: Commands,
) {
    let id = query.single();
    commands.entity(id).despawn_descendants();
}

fn try_to_run_after_error(mut reader: EventReader<AssetEvent<ScriptAsset>>,
                     state: Res<State<RunState>>,
                     mut next_state: ResMut<NextState<RunState>>) {
    if *state != RunState::Messages {
        return;
    }
    for e in reader.read() {
        if let AssetEvent::Modified { .. } = e {
            info!("Goto run state from error state.");
            next_state.set(RunState::Run);
        }
    }
}

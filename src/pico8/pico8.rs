use bevy::{
    asset::embedded_asset,
    audio::PlaybackMode,
    ecs::system::SystemParam,
    image::{ImageSampler, TextureAccessError},
    input::gamepad::GamepadConnectionEvent,
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    sprite::Anchor,
    text::TextLayoutInfo,
};
use tiny_skia::{self, FillRule, Paint, PathBuilder, Pixmap, Stroke};

#[cfg(feature = "scripting")]
use bevy_mod_scripting::{
    core::{
        asset::ScriptAsset,
        bindings::{function::from::FromScript, script_value::ScriptValue, WorldAccessGuard},
        docgen::typed_through::{ThroughTypeInfo, TypedThrough},
        error::InteropError,
    },
    lua::mlua::prelude::LuaError,
};
use bitvec::prelude::*;

use crate::{
    cursor::Cursor,
    pico8::{
        audio::{AudioBank, AudioCommand, SfxChannels, SfxDest},
        image::pixel_art_settings,
        keyboard::KeyInput,
        mouse::MouseInput,
        rand::Rand8,
        ClearEvent, Clearable, Gfx, GfxHandles, Map, PalMap, Palette,
    },
    DrawState, FillColor, N9Canvas, N9Color, Nano9Camera, PColor, ValueExt,
};

use std::{any::TypeId, borrow::Cow, f32::consts::PI};

pub const PICO8_PALETTE: &str = "embedded://nano9/pico8/pico-8-palette.png";
pub const PICO8_BORDER: &str = "embedded://nano9/pico8/rect-border.png";
pub const PICO8_FONT: &str = "embedded://nano9/pico8/pico-8.ttf";
pub const MAP_COLUMNS: u32 = 128;
pub const PICO8_SPRITE_SIZE: UVec2 = UVec2::new(8, 8);
pub const PICO8_TILE_COUNT: UVec2 = UVec2::new(16, 16);

const ANALOG_STICK_THRESHOLD: f32 = 0.1;
#[derive(Clone, Debug, Reflect)]
pub struct N9Font {
    pub handle: Handle<Font>,
}

#[derive(Resource, Debug, Reflect, Deref)]
pub struct Pico8Handle {
    #[deref]
    pub handle: Handle<Pico8Asset>,
    pub script_component: Option<Entity>,
}

impl From<Handle<Pico8Asset>> for Pico8Handle {
    fn from(handle: Handle<Pico8Asset>) -> Self {
        Self {
            handle,
            script_component: None,
        }
    }
}

#[derive(Clone, Asset, Debug, Reflect)]
pub struct Pico8Asset {
    #[cfg(feature = "scripting")]
    pub code: Option<Handle<ScriptAsset>>,
    pub(crate) palettes: Vec<Palette>,
    pub(crate) border: Handle<Image>,
    pub(crate) sprite_sheets: Vec<SpriteSheet>,
    pub(crate) maps: Vec<Map>,
    pub(crate) font: Vec<N9Font>,
    pub(crate) audio_banks: Vec<AudioBank>,
}

/// Pico8State's state.
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct Pico8State {
    #[reflect(ignore)]
    pub(crate) pal_map: PalMap,
    /// Current palette
    pub(crate) palette: usize,
    pub(crate) draw_state: DrawState,
}

#[derive(Reflect, Clone, Debug, Copy)]
pub enum Spr {
    /// Sprite at current spritesheet.
    Cur { sprite: usize },
    /// Sprite from given spritesheet.
    From { sprite: usize, sheet: usize },
    /// Set spritesheet.
    ///
    /// XXX: Not sure I like this.
    Set { sheet: usize },
}

#[cfg(feature = "scripting")]
impl FromScript for Spr {
    type This<'w> = Self;
    fn from_script(
        value: ScriptValue,
        _world: WorldAccessGuard<'_>,
    ) -> Result<Self::This<'_>, InteropError> {
        match value {
            ScriptValue::Float(f) => Ok(Spr::Cur { sprite: f as usize }),
            ScriptValue::Integer(n) => Ok(if n >= 0 {
                Spr::Cur { sprite: n as usize }
            } else {
                Spr::Set {
                    sheet: n.unsigned_abs() as usize,
                }
            }),
            ScriptValue::List(list) => {
                assert_eq!(list.len(), 2, "Expect two elements for spr.");
                let mut iter = list.into_iter().map(|v| match v {
                    ScriptValue::Integer(n) => Ok(n as usize),
                    x => Err(InteropError::external_error(Box::new(
                        Error::InvalidArgument(format!("{x:?}").into()),
                    ))),
                });
                let sprite = iter.next().expect("sprite index")?;
                let sheet = iter.next().expect("sheet index")?;
                Ok(Spr::From { sprite, sheet })
            }
            _ => Err(InteropError::impossible_conversion(TypeId::of::<Spr>())),
        }
    }
}

impl From<i64> for Spr {
    fn from(index: i64) -> Self {
        if index >= 0 {
            Spr::Cur {
                sprite: index as usize,
            }
        } else {
            Spr::Set {
                sheet: index.abs().saturating_sub(1) as usize,
            }
        }
    }
}

impl From<usize> for Spr {
    fn from(sprite: usize) -> Self {
        Spr::Cur { sprite }
    }
}

impl From<i32> for Spr {
    fn from(sprite: i32) -> Self {
        Spr::Cur { sprite: sprite as usize }
    }
}

impl From<(usize, usize)> for Spr {
    fn from((sprite, sheet): (usize, usize)) -> Self {
        Spr::From { sprite, sheet }
    }
}

#[derive(Debug, Clone, Reflect)]
pub enum SprHandle {
    Gfx(Handle<Gfx>),
    Image(Handle<Image>),
}

#[derive(Debug, Clone, Reflect)]
pub struct SpriteSheet {
    pub handle: SprHandle,
    pub layout: Handle<TextureAtlasLayout>,
    pub sprite_size: UVec2,
    pub flags: Vec<u8>,
}

#[derive(Event, Debug)]
struct UpdateCameraPos(Vec2);

#[derive(Default, Debug, Clone)]
pub struct Buttons {
    from: Option<Entity>,
    curr: BitArray<[u8; 1]>,
    last: BitArray<[u8; 1]>,
}

impl Buttons {
    pub fn btnp(&self, b: Option<u8>) -> Result<bool, Error> {
        match b {
            Some(b) => {
                let curr = self
                    .curr
                    .get(b as usize)
                    .map(|x| *x.as_ref())
                    .ok_or(Error::NoSuchButton(b))?;
                let last = self
                    .last
                    .get(b as usize)
                    .map(|x| *x.as_ref())
                    .ok_or(Error::NoSuchButton(b))?;
                Ok(curr && !last)
            }
            None => Ok((self.curr & (self.curr & !self.last)).any()),
        }
    }

    pub fn btn(&self, b: Option<u8>) -> Result<bool, Error> {
        match b {
            Some(b) => self
                .curr
                .get(b as usize)
                .map(|x| *x.as_ref())
                .ok_or(Error::NoSuchButton(b)),
            None => Ok(self.curr.any()),
        }
    }
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct PlayerInputs(Vec<Buttons>);

impl Default for PlayerInputs {
    fn default() -> Self {
        PlayerInputs(vec![Buttons::default(); 2])
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("no such {0:?}")]
    NoSuch(Cow<'static, str>),
    #[error("no asset {0:?} loaded")]
    NoAsset(Cow<'static, str>),
    // #[error("invalid {0:?}")]
    // Invalid(Cow<'static, str>),
    #[error("texture access error: {0}")]
    TextureAccess(#[from] TextureAccessError),
    #[error("no such button: {0}")]
    NoSuchButton(u8),
    #[error("invalid argument {0}")]
    InvalidArgument(Cow<'static, str>),
    #[error("unsupported {0}")]
    Unsupported(Cow<'static, str>),
    #[error("no sfx channel {0}")]
    NoChannel(u8),
    #[error("all sfx channels are busy")]
    ChannelsBusy,
    #[error("unsupported poke at address {0}")]
    UnsupportedPoke(usize),
    #[error("unsupported peek at address {0}")]
    UnsupportedPeek(usize),
    #[error("unsupported stat at address {0}")]
    UnsupportedStat(u8),
}

#[cfg(feature = "scripting")]
impl From<Error> for LuaError {
    fn from(e: Error) -> Self {
        LuaError::RuntimeError(format!("pico8 error: {e}"))
    }
}

#[derive(SystemParam)]
pub struct Pico8<'w, 's> {
    // TODO: Turn these image operations into triggers so that the Pico8 system
    // parameter will not preclude users from accessing images in their rust
    // systems.
    images: ResMut<'w, Assets<Image>>,
    pub state: ResMut<'w, Pico8State>,
    commands: Commands<'w, 's>,
    canvas: Res<'w, N9Canvas>,
    player_inputs: Res<'w, PlayerInputs>,
    sfx_channels: Res<'w, SfxChannels>,
    time: Res<'w, Time>,
    #[cfg(feature = "level")]
    tiled: crate::level::tiled::Level<'w, 's>,
    gfxs: ResMut<'w, Assets<Gfx>>,
    gfx_handles: ResMut<'w, GfxHandles>,
    rand8: Rand8<'w>,
    key_input: ResMut<'w, KeyInput>,
    mouse_input: ResMut<'w, MouseInput>,
    pico8_assets: ResMut<'w, Assets<Pico8Asset>>,
    pico8_handle: Res<'w, Pico8Handle>,
}

pub(crate) fn fill_input(
    mut connection_events: EventReader<GamepadConnectionEvent>,
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut player_inputs: ResMut<PlayerInputs>,
) {
    for event in connection_events.read() {
        info!("{event:?}");
        if event.connected() {
            match player_inputs
                .iter_mut()
                .find(|buttons| buttons.from.is_none())
            {
                Some(buttons) => buttons.from = Some(event.gamepad),
                None => player_inputs.push(Buttons {
                    from: Some(event.gamepad),
                    ..default()
                }),
            }
        } else {
            // disconnected
            match player_inputs
                .iter_mut()
                .find(|buttons| buttons.from == Some(event.gamepad))
            {
                Some(buttons) => buttons.from = None,
                None => {
                    warn!("Gamepad disconnected but not present in player inputs.");
                }
            }
        }
    }
    for (i, buttons) in player_inputs.iter_mut().enumerate() {
        buttons.last = buttons.curr;
        buttons.curr.fill(false);

        // buttons.curr.set(0, keys.pressed(KeyCode::ArrowLeft)
        for b in 0..=5 {
            let key_pressed = match i {
                0 => match b {
                    0 => keys.pressed(KeyCode::ArrowLeft),
                    1 => keys.pressed(KeyCode::ArrowRight),
                    2 => keys.pressed(KeyCode::ArrowUp),
                    3 => keys.pressed(KeyCode::ArrowDown),
                    4 => keys.any_pressed([
                        KeyCode::KeyZ,
                        KeyCode::KeyC,
                        KeyCode::KeyN,
                        KeyCode::NumpadSubtract,
                    ]),
                    5 => keys.any_pressed([
                        KeyCode::KeyX,
                        KeyCode::KeyV,
                        KeyCode::KeyM,
                        KeyCode::Numpad8,
                    ]),
                    _ => unreachable!(),
                },
                1 => match b {
                    0 => keys.pressed(KeyCode::KeyS),
                    1 => keys.pressed(KeyCode::KeyF),
                    2 => keys.pressed(KeyCode::KeyE),
                    3 => keys.pressed(KeyCode::KeyD),
                    4 => keys.any_pressed([KeyCode::ShiftLeft, KeyCode::Tab]),
                    5 => keys.any_pressed([KeyCode::KeyA, KeyCode::KeyQ]),
                    _ => unreachable!(),
                },
                _ => false,
            };
            let (button, dir_maybe) = match b {
                0 => (GamepadButton::DPadLeft, Some(Vec2::NEG_X)),
                1 => (GamepadButton::DPadRight, Some(Vec2::X)),
                2 => (GamepadButton::DPadUp, Some(Vec2::Y)),
                3 => (GamepadButton::DPadDown, Some(Vec2::NEG_Y)),
                4 => (GamepadButton::South, None),
                5 => (GamepadButton::East, None),
                _ => unreachable!(),
            };
            let button_pressed = buttons
                .from
                .and_then(|id| {
                    // We have a gamepad.
                    gamepads.get(id).ok().map(|gamepad| {
                        gamepad.pressed(button)
                            || dir_maybe
                                .map(|dir| gamepad.left_stick().dot(dir) > ANALOG_STICK_THRESHOLD)
                                .unwrap_or(false)
                    })
                })
                .unwrap_or(false);
            buttons.curr.set(b, key_pressed || button_pressed);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SfxCommand {
    Play(u8),
    Release,
    Stop,
}

impl From<u8> for SfxCommand {
    fn from(x: u8) -> Self {
        SfxCommand::Play(x)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum Radii {
    Radii(u32, u32),
    Radius(u32),
}

#[derive(Debug, Clone, Reflect)]
pub enum PropBy {
    Pos(Vec2),
    Rect(Rect),
    Name(Cow<'static, str>),
}

impl From<Vec2> for PropBy {
    fn from(v: Vec2) -> Self {
        PropBy::Pos(v)
    }
}

impl From<String> for PropBy {
    fn from(v: String) -> Self {
        PropBy::Name(v.into())
    }
}

// impl Default for PropBy {
//     fn default() -> Self {
//         PropBy::Pos(Vec2::ZERO)
//     }
// }

#[cfg(feature = "scripting")]
impl TypedThrough for PropBy {
    fn through_type_info() -> ThroughTypeInfo {
        ThroughTypeInfo::TypeInfo(<PropBy as bevy::reflect::Typed>::type_info())
    }
}

#[cfg(feature = "scripting")]
impl FromScript for PropBy {
    type This<'w> = Self;
    fn from_script(
        value: ScriptValue,
        _world: WorldAccessGuard<'_>,
    ) -> Result<Self::This<'_>, InteropError> {
        match value {
            ScriptValue::String(n) => Ok(PropBy::Name(n)),
            ScriptValue::List(l) => {
                let x = l.first().and_then(ValueExt::to_f32).unwrap_or(0.0);
                let y = l.get(1).and_then(ValueExt::to_f32).unwrap_or(0.0);
                Ok(PropBy::Pos(Vec2::new(x, y)))
            }
            ScriptValue::Map(v) => {
                let x = v.get("x").and_then(ValueExt::to_f32).unwrap_or(0.0);
                let y = v.get("y").and_then(ValueExt::to_f32).unwrap_or(0.0);
                let w = v.get("width").and_then(ValueExt::to_f32);
                let h = v.get("height").and_then(ValueExt::to_f32);
                if w.is_some() && h.is_some() {
                    Ok(PropBy::Rect(Rect::from_corners(
                        Vec2::new(x, y),
                        Vec2::new(x + w.unwrap(), y + h.unwrap()),
                    )))
                } else {
                    Ok(PropBy::Pos(Vec2::new(x, y)))
                }
            }
            _ => Err(InteropError::impossible_conversion(TypeId::of::<PropBy>())),
        }
    }
}

impl From<Radii> for UVec2 {
    fn from(r: Radii) -> UVec2 {
        match r {
            Radii::Radii(r1, r2) => UVec2::new(r1, r2),
            Radii::Radius(r) => UVec2::new(r, r),
        }
    }
}

/// Negates y IF the feature "negate-y" is enabled.
#[inline]
pub fn negate_y(y: f32) -> f32 {
    if cfg!(feature = "negate-y") {
        -y
    } else {
        y
    }
}

impl Pico8<'_, '_> {
    /// sspr( sx, sy, sw, sh, dx, dy, [dw,] [dh,] [flip_x,] [flip_y,] [sheet_index])
    pub fn sspr(
        &mut self,
        sprite_rect: Rect,
        screen_pos: Vec2,
        screen_size: Option<Vec2>,
        flip: Option<BVec2>,
        sheet_index: Option<usize>,
    ) -> Result<Entity, Error> {
        let screen_pos = self.state.draw_state.apply_camera_delta(screen_pos);
        let x = screen_pos.x;
        let y = screen_pos.y;
        let flip = flip.unwrap_or_default();
        let sheet_index = sheet_index.unwrap_or(0);
        let sheet = self
            .pico8_asset()?
            .sprite_sheets
            .get(sheet_index)
            .ok_or(Error::NoSuch(format!("image {sheet_index}").into()))?
            .clone();
        let sprite = Sprite {
            image: match sheet.handle {
                SprHandle::Image(handle) => handle,
                SprHandle::Gfx(handle) => {
                    // XXX: Consider copying palettes to state to avoid cloning.
                    let palette = &self.palette(None)?.clone();
                    self.gfx_handles.get_or_create(
                        &palette,
                        &self.state.pal_map,
                        None,
                        &handle,
                        &self.gfxs,
                        &mut self.images,
                    )?
                }
            },
            anchor: Anchor::TopLeft,
            rect: Some(sprite_rect),
            custom_size: screen_size,
            flip_x: flip.x,
            flip_y: flip.y,
            ..default()
        };
        let clearable = Clearable::default();
        Ok(self
            .commands
            .spawn((
                Name::new("spr"),
                sprite,
                Transform::from_xyz(x, negate_y(y), clearable.suggest_z()),
                clearable,
            ))
            .id())
    }

    fn pico8_asset(&self) -> Result<&Pico8Asset, Error> {
        self.pico8_assets
            .get(&self.pico8_handle.handle)
            .ok_or(Error::NoSuch("Pico8Asset".into()))
    }

    fn sprite_sheet(&self, sheet_index: Option<usize>) -> Result<&SpriteSheet, Error> {
        let index = sheet_index.unwrap_or(0);
        self.pico8_asset()?
            .sprite_sheets
            .get(index)
            .ok_or(Error::NoSuch(format!("image index {index}").into()))
    }

    fn sprite_map(&self, map_index: Option<usize>) -> Result<&Map, Error> {
        let index = map_index.unwrap_or(0);
        self.pico8_asset()?
            .maps
            .get(index)
            .ok_or(Error::NoSuch(format!("map index {index}").into()))
    }

    fn pico8_asset_mut(&mut self) -> Result<&mut Pico8Asset, Error> {
        self.pico8_assets
            .get_mut(&self.pico8_handle.handle)
            .ok_or(Error::NoSuch("Pico8Asset".into()))
    }

    fn sprite_sheet_mut(&mut self, sheet_index: Option<usize>) -> Result<&mut SpriteSheet, Error> {
        let index = sheet_index.unwrap_or(0);
        self.pico8_asset_mut()?
            .sprite_sheets
            .get_mut(index)
            .ok_or(Error::NoSuch(format!("image index {index}").into()))
    }

    fn sprite_map_mut(&mut self, map_index: Option<usize>) -> Result<&mut Map, Error> {
        let index = map_index.unwrap_or(0);
        self.pico8_asset_mut()?
            .maps
            .get_mut(index)
            .ok_or(Error::NoSuch(format!("map index {index}").into()))
    }

    /// spr(n, [x,] [y,] [w,] [h,] [flip_x,] [flip_y])
    pub fn spr(
        &mut self,
        spr: impl Into<Spr>,
        pos: Vec2,
        size: Option<Vec2>,
        flip: Option<BVec2>,
        turns: Option<f32>,
    ) -> Result<Entity, Error> {
        let pos = self.state.draw_state.apply_camera_delta(pos);
        let x = pos.x;
        let y = pos.y;
        let flip = flip.unwrap_or_default();
        let (sprites, index): (&SpriteSheet, usize) = match spr.into() {
            Spr::Cur { sprite } => (self.sprite_sheet(None)?, sprite),
            Spr::From { sheet, sprite } => (self.sprite_sheet(Some(sheet))?, sprite),
            Spr::Set { sheet } => {
                todo!()
                // self.state.sprite_sheets.pos = sheet;
                // return Ok(Entity::PLACEHOLDER);
            }
        };
        let atlas = TextureAtlas {
            layout: sprites.layout.clone(),
            index,
        };
        let rect = size.map(|v| Rect {
            min: Vec2::ZERO,
            max: sprites.sprite_size.as_vec2() * v,
        });
        let pixel_size = sprites.sprite_size.as_vec2() * size.unwrap_or(Vec2::ONE) / 2.0;

        let image = match sprites.handle.clone() {
            SprHandle::Image(handle) => handle,
            SprHandle::Gfx(handle) => {
                let palette = &self.palette(None)?.clone();
                self.gfx_handles.get_or_create(
                    palette,
                    &self.state.pal_map,
                    None,
                    &handle,
                    &self.gfxs,
                    &mut self.images,
                )?
            }
        };
        let mut sprite = {
            Sprite {
                image,
                anchor: Anchor::TopLeft,
                texture_atlas: Some(atlas),
                rect,
                flip_x: flip.x,
                flip_y: flip.y,
                ..default()
            }
        };
        let clearable = Clearable::default();
        let mut transform = Transform::from_xyz(x, negate_y(y), clearable.suggest_z());
        if let Some(turns) = turns {
            transform.translation.x += pixel_size.x;
            transform.translation.y += negate_y(pixel_size.y);
            sprite.anchor = Anchor::Center;
            transform.rotation = Quat::from_rotation_z(turns * 2.0 * PI);
        }
        Ok(self
            .commands
            .spawn((Name::new("spr"), sprite, transform, clearable))
            .id())
    }

    // cls([n])
    pub fn cls(&mut self, color: Option<impl Into<PColor>>) -> Result<(), Error> {
        trace!("cls");
        let c = self.get_color(color.map(|x| x.into()).unwrap_or(Color::BLACK.into()))?;
        self.state.draw_state.clear_screen();
        let image = self
            .images
            .get_mut(&self.canvas.handle)
            .ok_or(Error::NoAsset("canvas".into()))?;
        for i in 0..image.width() {
            for j in 0..image.height() {
                image.set_color_at(i, j, c)?;
            }
        }
        self.commands.send_event(ClearEvent::default());
        Ok(())
    }

    pub fn pset(&mut self, pos: UVec2, color: impl Into<N9Color>) -> Result<(), Error> {
        let c = self.get_color(color.into())?;
        let image = self
            .images
            .get_mut(&self.canvas.handle)
            .ok_or(Error::NoAsset("canvas".into()))?;
        image.set_color_at(pos.x, pos.y, c)?;
        Ok(())
    }

    pub fn sset(
        &mut self,
        pos: UVec2,
        color: Option<N9Color>,
        sheet_index: Option<usize>,
    ) -> Result<(), Error> {
        let color = color.unwrap_or(N9Color::Pen);
        let sheet = self.sprite_sheet(sheet_index)?;
        match sheet.handle.clone() {
            SprHandle::Gfx(handle) => {
                let gfx = self
                    .gfxs
                    .get_mut(&handle)
                    .ok_or(Error::NoSuch("Gfx".into()))?;
                gfx.set(
                    pos.x as usize,
                    pos.y as usize,
                    match color.into_pcolor(&self.state.draw_state.pen) {
                        PColor::Palette(n) => Ok(n as u8),
                        PColor::Color(_) => Err(Error::InvalidArgument(
                            "Cannot write pen `Color` to Gfx asset".into(),
                        )),
                    }?,
                );
            }
            SprHandle::Image(handle) => {
                let c = self.get_color(color)?;
                let image = self
                    .images
                    .get_mut(&handle)
                    .ok_or(Error::NoAsset("canvas".into()))?;
                image.set_color_at(pos.x, pos.y, c)?;
            }
        }
        Ok(())
    }

    pub fn sget(
        &mut self,
        pos: UVec2,
        sheet_index: Option<usize>,
    ) -> Result<Option<PColor>, Error> {
        let sheet = self.sprite_sheet(sheet_index)?;
        Ok(match &sheet.handle {
            SprHandle::Gfx(handle) => {
                let gfx = self.gfxs.get(handle).ok_or(Error::NoSuch("Gfx".into()))?;
                gfx.get(pos.x as usize, pos.y as usize)
                    .map(|i| PColor::Palette(i as usize))
            }
            SprHandle::Image(handle) => {
                let image = self
                    .images
                    .get(handle)
                    .ok_or(Error::NoAsset("canvas".into()))?;
                Some(PColor::Color(image.get_color_at(pos.x, pos.y)?.into()))
            }
        })
    }

    pub fn rectfill(
        &mut self,
        upper_left: Vec2,
        lower_right: Vec2,
        color: Option<impl Into<FillColor>>,
    ) -> Result<Entity, Error> {
        let upper_left = self.state.draw_state.apply_camera_delta(upper_left);
        let lower_right = self.state.draw_state.apply_camera_delta(lower_right);
        let size = (lower_right - upper_left) + Vec2::ONE;
        let clearable = Clearable::default();
        let color = color.map(|x| x.into());
        let id = self
            .commands
            .spawn((
                Name::new("rectfill"),
                if let Some(fill_pat) = &self.state.draw_state.fill_pat {
                    let image =
                            // {
                            //     let a = Gfx::<1>::from_vec(8,8,
                            //                                vec![
                            //                                    0b10000000,
                            //                                    0b01000000,
                            //                                    0b00100000,
                            //                                    0b00010000,
                            //                                    0b00001000,
                            //                                    0b00000100,
                            //                                    0b00000010,
                            //                                    0b00000001,
                            //                                ]);
                            //     a.mirror_horizontal().to_image(|i, _, pixel_bytes| {
                            //         pixel_bytes.copy_from_slice(&PALETTE[i as usize]);
                            //     })
                            // }
                            fill_pat.to_image(8, 8, |bit, _pixel_index, pixel_bytes| {
                                let c: Option<PColor> = if bit {
                                    color.and_then(|x| x.on())
                                } else {
                                    color.map(|x| x.off()).or(Some(self.state.draw_state.pen))
                                };
                                if let Some(c) = c {
                                    // c.map(&self.state.pal_map).write_color(&PALETTE, pixel_bytes);
                                    let _ = c.write_color(
                                        &self.pico8_asset()?.palettes[self.state.palette].data,
                                        &self.state.pal_map,
                                        pixel_bytes,
                                    );
                                }
                                Ok::<(), Error>(())
                            })?;
                    Sprite {
                        anchor: Anchor::TopLeft,
                        // NOTE: Technically we only need a 4x4 image. However, this generates a warning.
                        //
                        // ```
                        // WARN bevy_sprite::texture_slice: One of your tiled
                        // textures has generated 1089 slices. You might want to
                        // use higher stretch values to avoid a great
                        // performance cost
                        // ```
                        //
                        // So we generate an 8x8 to avoid that.
                        image: self.images.add(image),
                        custom_size: Some(size),
                        image_mode: SpriteImageMode::Tiled {
                            tile_x: true,
                            tile_y: true,
                            stretch_value: 1.0,
                        },
                        ..default()
                    }
                } else {
                    let c =
                        self.get_color(color.map(|x| x.off().into()).unwrap_or(N9Color::Pen))?;
                    Sprite {
                        color: c,
                        anchor: Anchor::TopLeft,
                        custom_size: Some(size),
                        ..default()
                    }
                },
                Transform::from_xyz(upper_left.x, negate_y(upper_left.y), clearable.suggest_z()),
                clearable,
            ))
            .id();
        self.state.draw_state.mark_drawn();
        Ok(id)
    }

    pub fn rect(
        &mut self,
        upper_left: Vec2,
        lower_right: Vec2,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let upper_left = self.state.draw_state.apply_camera_delta(upper_left);
        let lower_right = self.state.draw_state.apply_camera_delta(lower_right);
        let c = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let size = (lower_right - upper_left) + Vec2::ONE;
        let clearable = Clearable::default();
        let id = self
            .commands
            .spawn((
                Name::new("rect"),
                Sprite {
                    image: self.pico8_asset()?.border.clone(),
                    color: c,
                    anchor: Anchor::TopLeft,
                    custom_size: Some(size),
                    image_mode: SpriteImageMode::Sliced(TextureSlicer {
                        border: BorderRect::square(1.0),
                        center_scale_mode: SliceScaleMode::Stretch,
                        sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1.0 },
                        ..default()
                    }),
                    ..default()
                },
                Transform::from_xyz(upper_left.x, negate_y(upper_left.y), clearable.suggest_z()),
                clearable,
            ))
            .id();
        self.state.draw_state.mark_drawn();
        Ok(id)
    }

    fn palette(&self, index: Option<usize>) -> Result<&Palette, Error> {
        Ok(self
            .pico8_asset()?
            .palettes
            .get(index.unwrap_or(self.state.palette))
            .ok_or(Error::NoSuch("palette".into()))?)
    }

    pub fn map(
        &mut self,
        map_pos: UVec2,
        mut screen_start: Vec2,
        size: UVec2,
        mask: Option<u8>,
        map_index: Option<usize>,
    ) -> Result<Entity, Error> {
        screen_start = self.state.draw_state.apply_camera_delta(screen_start);
        if cfg!(feature = "negate-y") {
            screen_start.y = -screen_start.y;
        }
        match self.sprite_map(map_index)?.clone() {
            Map::P8(map) => {
                let palette = self.palette(None)?.clone();

                let sprite_sheets = &self.pico8_asset()?.sprite_sheets.clone();
                map.map(
                    map_pos,
                    screen_start,
                    size,
                    mask,
                    &sprite_sheets,
                    &mut self.commands,
                    |handle| {
                        self.gfx_handles.get_or_create(
                            &palette,
                            &self.state.pal_map,
                            None,
                            handle,
                            &self.gfxs,
                            &mut self.images,
                        )
                    },
                )
            }
            #[cfg(feature = "level")]
            Map::Level(map) => Ok(map.map(screen_start, 0, &mut self.commands)),
        }
    }

    pub fn btnp(&self, b: Option<u8>, player: Option<u8>) -> Result<bool, Error> {
        let Some(buttons) = self.player_inputs.get(player.unwrap_or(0) as usize) else {
            return Err(Error::NoSuch("player".into()));
        };
        buttons.btnp(b)
    }

    pub fn btn(&self, b: Option<u8>, player: Option<u8>) -> Result<bool, Error> {
        let Some(buttons) = self.player_inputs.get(player.unwrap_or(0) as usize) else {
            return Err(Error::NoSuch("player".into()));
        };
        buttons.btn(b)
    }

    /// print(text, [x,] [y,] [color,] [font_size])
    ///
    /// Print the given text. The Lua `print()` function will return the new x
    /// value. This function only returns the entity. To recover the new x
    /// value, one can call the `cursor().x` function.
    pub fn print(
        &mut self,
        text: impl Into<String>,
        pos: Option<Vec2>,
        color: Option<N9Color>,
        font_size: Option<f32>,
        font_index: Option<usize>,
    ) -> Result<Entity, Error> {
        let text = text.into();
        let id = self.commands.spawn_empty().id();
        self.commands.queue(move |world: &mut World| {
            Self::print_world(world, Some(id), text, pos, color, font_size, font_index);
        });
        Ok(id)
    }

    pub(crate) fn print_world(
        world: &mut World,
        dest: Option<Entity>,
        text: String,
        pos: Option<Vec2>,
        color: Option<N9Color>,
        font_size: Option<f32>,
        font_index: Option<usize>,
    ) -> Result<f32, Error> {
        let (id, add_newline) =
            Self::pre_print_world(world, dest, text, pos, color, font_size, font_index)?;
        world
            .run_system_cached(bevy::text::update_text2d_layout)
            .expect("update_text2d_layout");
        world
            .run_system_cached_with(Self::post_print_world, (id, add_newline))
            .expect("post_print_world")
    }

    fn post_print_world(
        In((id, add_newline)): In<(Entity, bool)>,
        query: Query<(&Transform, &TextLayoutInfo)>,
        mut state: ResMut<Pico8State>,
    ) -> Result<f32, Error> {
        let (transform, text_layout) = query
            .get(id)
            .map_err(|_| Error::NoSuch("text layout".into()))?;
        let pos = &transform.translation;
        if add_newline {
            state.draw_state.print_cursor.x = pos.x;
            state.draw_state.print_cursor.y = negate_y(pos.y) + text_layout.size.y;
        } else {
            state.draw_state.print_cursor.x = pos.x + text_layout.size.x;
        }
        state.draw_state.mark_drawn();
        Ok(pos.x + text_layout.size.x)
    }

    fn pre_print_world(
        world: &mut World,
        entity: Option<Entity>,
        mut text: String,
        pos: Option<Vec2>,
        color: Option<N9Color>,
        font_size: Option<f32>,
        font_index: Option<usize>,
    ) -> Result<(Entity, bool), Error> {
        let assets = world
            .get_resource::<Assets<Pico8Asset>>()
            .expect("Pico8Assets");
        let state = world.get_resource::<Pico8State>().expect("Pico8State");
        let pico8_handle = world.get_resource::<Pico8Handle>().expect("Pico8Handle");
        let pico8_asset = assets
            .get(&pico8_handle.handle)
            .ok_or(Error::NoSuch("Pico8Asset".into()))?;
        let font = pico8_asset
            .font
            .get(font_index.unwrap_or(0))
            .ok_or(Error::NoSuch("font".into()))?
            .handle
            .clone();

        let c = pico8_asset.get_color(
            color
                .unwrap_or(N9Color::Pen)
                .into_pcolor(&state.draw_state.pen),
            state.palette,
        )?;
        // XXX: Should the camera delta apply to the print cursor position?
        let pos = pos
            .map(|p| state.draw_state.apply_camera_delta(p))
            .unwrap_or_else(|| {
                Vec2::new(
                    state.draw_state.print_cursor.x,
                    state.draw_state.print_cursor.y,
                )
            });
        // pos =
        let clearable = Clearable::default();
        let add_newline = if text.ends_with('\0') {
            text.pop();
            false
        } else {
            true
        };
        let font_size = font_size.unwrap_or(5.0);
        let z = clearable.suggest_z();
        let id = entity.unwrap_or_else(|| world.spawn_empty().id());
        world.entity_mut(id).insert((
            Name::new("print"),
            Transform::from_xyz(pos.x, negate_y(pos.y), z),
            Text2d::new(text),
            Visibility::default(),
            TextColor(c),
            TextFont {
                font,
                font_smoothing: bevy::text::FontSmoothing::None,
                font_size,
            },
            Anchor::TopLeft,
            clearable,
        ));
        Ok((id, add_newline))
    }

    pub fn exit(&mut self, error: Option<u8>) {
        self.commands.send_event(match error {
            Some(n) => std::num::NonZero::new(n)
                .map(AppExit::Error)
                .unwrap_or(AppExit::Success),
            None => AppExit::Success,
        });
    }

    // sfx( n, [channel,] [offset,] [length] )
    pub fn sfx(
        &mut self,
        n: impl Into<SfxCommand>,
        channel: Option<u8>,
        offset: Option<u8>,
        length: Option<u8>,
        bank: Option<u8>,
    ) -> Result<(), Error> {
        assert!(offset.is_none(), "offset not implemented");
        assert!(length.is_none(), "length not implemented");
        let n = n.into();
        let bank = bank.unwrap_or(0);
        match n {
            SfxCommand::Release => {
                if let Some(chan) = channel {
                    // let chan = self.sfx_channels[chan as usize];
                    self.commands
                        .queue(AudioCommand::Release(SfxDest::Channel(chan)));
                } else {
                    self.commands.queue(AudioCommand::Release(SfxDest::Any));
                }
            }
            SfxCommand::Stop => {
                if let Some(chan) = channel {
                    // let chan = self.sfx_channels[chan as usize];
                    self.commands.queue(AudioCommand::Stop(
                        SfxDest::Channel(chan),
                        Some(PlaybackMode::Remove),
                    ));
                } else {
                    self.commands
                        .queue(AudioCommand::Stop(SfxDest::All, Some(PlaybackMode::Remove)));
                }
            }
            SfxCommand::Play(n) => {
                let sfx = self
                    .pico8_asset()?
                    .audio_banks
                    .get(bank as usize)
                    .ok_or(Error::NoAsset(format!("bank {bank}").into()))?
                    .get(n as usize)
                    .ok_or(Error::NoAsset(format!("sfx {n}").into()))?
                    .clone();

                if let Some(chan) = channel {
                    // let chan = self.sfx_channels[chan as usize];
                    self.commands.queue(AudioCommand::Play(
                        sfx,
                        SfxDest::Channel(chan),
                        PlaybackSettings::REMOVE,
                    ));
                } else {
                    self.commands.queue(AudioCommand::Play(
                        sfx,
                        SfxDest::Any,
                        PlaybackSettings::REMOVE,
                    ));
                }
            }
        }
        Ok(())
    }

    // music( n, [facems,] [channelmask,] )
    pub fn music(
        &mut self,
        n: impl Into<SfxCommand>,
        fade_ms: Option<u32>,
        channel_mask: Option<u8>,
        bank: Option<u8>,
    ) -> Result<(), Error> {
        let n = n.into();
        let bank = bank.unwrap_or(0);
        return Ok(());
        match n {
            SfxCommand::Release => {
                panic!("Music does not accept a release command.");
            }
            SfxCommand::Stop => {
                // if let Some(chan) = channel {
                //     let chan = self.sfx_channels[chan as usize];
                //     self.commands
                //         .queue(AudioCommand::Stop(SfxDest::Channel(chan), Some(PlaybackMode::Loop)));
                // } else {
                self.commands
                    .queue(AudioCommand::Stop(SfxDest::All, Some(PlaybackMode::Loop)));
                // }
            }
            SfxCommand::Play(n) => {
                let sfx = self
                    .pico8_asset()?
                    .audio_banks
                    .get(bank as usize)
                    .ok_or(Error::NoSuch(format!("audio bank {bank}").into()))?
                    .get(n as usize)
                    .ok_or(Error::NoAsset(format!("music {n}").into()))?
                    .clone();

                if let Some(mask) = channel_mask {
                    self.commands.queue(AudioCommand::Play(
                        sfx,
                        SfxDest::ChannelMask(mask),
                        PlaybackSettings::LOOP,
                    ));
                } else {
                    self.commands.queue(AudioCommand::Play(
                        sfx,
                        SfxDest::Any,
                        PlaybackSettings::LOOP,
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn fget(&self, index: Option<usize>, flag_index: Option<u8>) -> Result<u8, Error> {
        if index.is_none() {
            return Ok(0);
        }
        let index = index.unwrap();
        let flags = &self.sprite_sheet(None)?.flags;
        if let Some(v) = flags.get(index) {
            match flag_index {
                Some(flag_index) => {
                    if v & (1 << flag_index) != 0 {
                        Ok(1)
                    } else {
                        Ok(0)
                    }
                }
                None => Ok(*v),
            }
        } else {
            if flags.is_empty() {
                warn_once!("No flags present.");
            } else {
                warn!(
                    "Requested flag at {index}. There are only {} flags.",
                    flags.len()
                );
            }
            Ok(0)
        }
    }

    pub fn fset(&mut self, index: usize, flag_index: Option<u8>, value: u8) -> Result<(), Error> {
        let flags = &mut self.sprite_sheet_mut(None)?.flags;
        Ok(match flag_index {
            Some(flag_index) => {
                if value != 0 {
                    // Set the bit.
                    flags[index] |= 1 << flag_index;
                } else {
                    // Unset the bit.
                    flags[index] &= !(1 << flag_index);
                }
            }
            None => {
                flags[index] = value;
            }
        })
    }

    #[cfg(feature = "level")]
    /// Get properties
    pub fn mgetp(
        &self,
        prop_by: PropBy,
        map_index: Option<usize>,
        layer_index: Option<usize>,
    ) -> Option<tiled::Properties> {
        let map: &Map = self.sprite_map(map_index).ok()?;
        match *map {
            Map::P8(ref _map) => None,

            #[cfg(feature = "level")]
            Map::Level(ref map) => self.tiled.mgetp(map, prop_by, map_index, layer_index),
        }
    }

    pub fn mget(
        &self,
        pos: Vec2,
        map_index: Option<usize>,
        layer_index: Option<usize>,
    ) -> Option<usize> {
        let map: &Map = self.sprite_map(map_index).ok()?;
        match *map {
            Map::P8(ref map) => {
                Some(map[(pos.x as u32 + pos.y as u32 * MAP_COLUMNS) as usize] as usize)
            }

            #[cfg(feature = "level")]
            Map::Level(ref map) => self.tiled.mget(map, pos, map_index, layer_index),
        }
    }

    pub fn mset(
        &mut self,
        pos: Vec2,
        sprite_index: usize,
        map_index: Option<usize>,
        layer_index: Option<usize>,
    ) -> Result<(), Error> {
        let map = self.sprite_map_mut(map_index)?;
        match map {
            Map::P8(ref mut map) => map
                .get_mut((pos.x as u32 + pos.y as u32 * MAP_COLUMNS) as usize)
                .map(|value| *value = sprite_index as u8)
                .ok_or(Error::NoSuch("map entry".into())),
            #[cfg(feature = "level")]
            Map::Level(ref mut map) => {
                todo!()
                // self.tiled
                //     .mset(map, pos, sprite_index, map_index, layer_index)
            }
        }
    }

    pub fn sub(string: &str, start: isize, end: Option<isize>) -> String {
        let count = string.chars().count() as isize;
        let start = if start < 0 {
            (count - start - 1) as usize
        } else {
            (start - 1) as usize
        };
        match end {
            Some(end) => {
                let end = if end < 0 {
                    (count - end) as usize
                } else {
                    end as usize
                };
                if start <= end {
                    string.chars().skip(start).take(end - start).collect()
                    // BUG: This cuts unicode boundaries.
                    // Ok(string[start..end].to_string())
                } else {
                    String::new()
                }
            }
            None => string.chars().skip(start).collect(),
        }
    }

    pub fn time(&self) -> f32 {
        self.time.elapsed_secs()
    }

    pub fn camera(&mut self, pos: Option<Vec2>) -> Vec2 {
        if let Some(pos) = pos {
            let last = std::mem::replace(&mut self.state.draw_state.camera_position, pos);
            if let Some(ref mut delta) = &mut self.state.draw_state.camera_position_delta {
                // Do not move the camera. Something has already been drawn.
                // Accumulate the delta.
                *delta += last - pos;
            } else {
                // info!("Update actual camera position");
                // We haven't drawn anything yet. Move the actual camera.
                self.commands.trigger(UpdateCameraPos(pos));
            }
            last
        } else {
            self.state.draw_state.camera_position
        }
    }

    pub fn line(&mut self, a: IVec2, b: IVec2, color: Option<N9Color>) -> Result<Entity, Error> {
        let a = self.state.draw_state.apply_camera_delta_ivec2(a);
        let b = self.state.draw_state.apply_camera_delta_ivec2(b);
        let color = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let min = a.min(b);
        let delta = b - a;
        let size = UVec2::new(delta.x.unsigned_abs(), delta.y.unsigned_abs()) + UVec2::ONE;
        let mut image = Image::new_fill(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0u8, 0u8, 0u8, 0u8],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        let c = a - min;
        let d = b - min;
        for (x, y) in
            bresenham::Bresenham::new((c.x as isize, c.y as isize), (d.x as isize, d.y as isize))
        {
            image.set_color_at(x as u32, y as u32, Color::WHITE)?;
        }
        let handle = self.images.add(image);
        let clearable = Clearable::default();
        let id = self
            .commands
            .spawn((
                Name::new("line"),
                Sprite {
                    image: handle,
                    color,
                    anchor: Anchor::TopLeft,
                    custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                    ..default()
                },
                Transform::from_xyz(min.x as f32, negate_y(min.y as f32), clearable.suggest_z()),
                clearable,
            ))
            .id();
        Ok(id)
    }

    #[cfg(feature = "scripting")]
    pub fn rnd(&mut self, value: Option<ScriptValue>) -> ScriptValue {
        self.rand8.rnd(value)
    }

    pub fn srand(&mut self, seed: u64) {
        self.rand8.srand(seed)
    }

    pub fn circfill(
        &mut self,
        pos: IVec2,
        r: impl Into<UVec2>,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let pos = self.state.draw_state.apply_camera_delta_ivec2(pos);
        let color = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let r: UVec2 = r.into();
        let size: UVec2 = r * UVec2::splat(2) + UVec2::ONE;
        let mut pixmap = Pixmap::new(size.x, size.y).expect("pixmap");
        let oval =
            tiny_skia::Rect::from_ltrb(0.0, 0.0, size.x as f32, size.y as f32).expect("circ rect");
        let path = PathBuilder::from_oval(oval).expect("circ path");
        let mut paint = Paint::default();
        paint.anti_alias = false;
        paint.set_color_rgba8(255, 255, 255, 255);
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            tiny_skia::Transform::identity(),
            None,
        );

        let mut image = Image::new(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixmap.take(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        let handle = self.images.add(image);
        let clearable = Clearable::default();
        let offset = 0.5;
        let id = self
            .commands
            .spawn((
                Name::new("circfill"),
                Sprite {
                    image: handle,
                    color,
                    anchor: Anchor::Custom(Vec2::new(
                        -offset / size.x as f32,
                        offset / size.y as f32,
                    )),
                    custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                    ..default()
                },
                Transform::from_xyz(pos.x as f32, negate_y(pos.y as f32), clearable.suggest_z()),
                clearable,
            ))
            .id();
        self.state.draw_state.mark_drawn();
        Ok(id)
    }

    pub fn circ(
        &mut self,
        pos: IVec2,
        r: impl Into<UVec2>,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let pos = self.state.draw_state.apply_camera_delta_ivec2(pos);
        let color = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let r: UVec2 = r.into();
        let size: UVec2 = r * UVec2::splat(2) + UVec2::ONE;
        let mut pixmap = Pixmap::new(size.x, size.y).expect("pixmap");
        let oval =
            tiny_skia::Rect::from_ltrb(0.0, 0.0, size.x as f32, size.y as f32).expect("circ rect");
        let path = PathBuilder::from_oval(oval).expect("circ path");
        let mut paint = Paint::default();
        paint.anti_alias = false;
        paint.set_color_rgba8(255, 255, 255, 255);
        let mut stroke = Stroke::default();
        stroke.width = 0.0;
        pixmap.stroke_path(
            &path,
            &paint,
            &stroke,
            tiny_skia::Transform::identity(),
            None,
        );

        let mut image = Image::new(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixmap.take(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );
        image.sampler = ImageSampler::nearest();

        let offset = 0.5;
        let handle = self.images.add(image);
        let clearable = Clearable::default();
        let id = self
            .commands
            .spawn((
                Name::new("circ"),
                Sprite {
                    image: handle,
                    color,
                    anchor: Anchor::Custom(Vec2::new(
                        -offset / size.x as f32,
                        offset / size.y as f32,
                    )),
                    custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                    ..default()
                },
                Transform::from_xyz(pos.x as f32, negate_y(pos.y as f32), clearable.suggest_z()),
                clearable,
            ))
            .id();
        self.state.draw_state.mark_drawn();
        Ok(id)
    }

    pub fn ovalfill(
        &mut self,
        upper_left: IVec2,
        lower_right: IVec2,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let upper_left = self.state.draw_state.apply_camera_delta_ivec2(upper_left);
        let lower_right = self.state.draw_state.apply_camera_delta_ivec2(lower_right);
        let color = self.get_color(color.unwrap_or(N9Color::Pen))?;
        // let min = a.min(b);
        let size: UVec2 = ((lower_right - upper_left) + IVec2::ONE)
            .try_into()
            .unwrap();
        // // let size = UVec2::new((a.x - b.x).abs() + 1,
        // //                       (a.y - b.y).abs() + 1);
        // let size = UVec2::new(delta.x.abs() as u32, delta.y.abs() as u32) + UVec2::ONE;
        // dbg!(a, b, size);
        let mut pixmap = Pixmap::new(size.x, size.y).expect("pixmap");
        let oval =
            tiny_skia::Rect::from_ltrb(0.0, 0.0, size.x as f32, size.y as f32).expect("oval rect");
        let path = PathBuilder::from_oval(oval).expect("oval path");
        let mut paint = Paint::default();
        paint.anti_alias = false;
        paint.set_color_rgba8(255, 255, 255, 255);
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            tiny_skia::Transform::identity(),
            None,
        );

        let mut image = Image::new(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixmap.take(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        let handle = self.images.add(image);
        let clearable = Clearable::default();
        let id = self
            .commands
            .spawn((
                Name::new("ovalfill"),
                Sprite {
                    image: handle,
                    color,
                    anchor: Anchor::TopLeft,
                    custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                    ..default()
                },
                Transform::from_xyz(
                    upper_left.x as f32,
                    negate_y(upper_left.y as f32),
                    clearable.suggest_z(),
                ),
                clearable,
            ))
            .id();
        self.state.draw_state.mark_drawn();
        Ok(id)
    }

    pub fn oval(
        &mut self,
        upper_left: IVec2,
        lower_right: IVec2,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let upper_left = self.state.draw_state.apply_camera_delta_ivec2(upper_left);
        let lower_right = self.state.draw_state.apply_camera_delta_ivec2(lower_right);
        let color = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let size: UVec2 = ((lower_right - upper_left) + IVec2::ONE)
            .try_into()
            .unwrap();
        let mut pixmap = Pixmap::new(size.x, size.y).expect("pixmap");
        let oval =
            tiny_skia::Rect::from_ltrb(0.0, 0.0, size.x as f32, size.y as f32).expect("oval rect");
        let path = PathBuilder::from_oval(oval).expect("oval path");
        let mut paint = Paint::default();
        paint.anti_alias = false;
        paint.set_color_rgba8(255, 255, 255, 255);
        let mut stroke = Stroke::default();
        stroke.width = 0.0;
        pixmap.stroke_path(
            &path,
            &paint,
            &stroke,
            tiny_skia::Transform::identity(),
            None,
        );

        let mut image = Image::new(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixmap.take(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );

        image.sampler = ImageSampler::nearest();
        let handle = self.images.add(image);
        let clearable = Clearable::default();
        let id = self
            .commands
            .spawn((
                Name::new("oval"),
                Sprite {
                    image: handle,
                    color,
                    anchor: Anchor::TopLeft,
                    custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                    ..default()
                },
                Transform::from_xyz(
                    upper_left.x as f32,
                    negate_y(upper_left.y as f32),
                    clearable.suggest_z(),
                ),
                clearable,
            ))
            .id();
        self.state.draw_state.mark_drawn();
        Ok(id)
    }

    // Set a sprite.
    // pub fn sset(&mut self, id: Entity, sprite_index: usize) {
    //     self.commands.queue(move |world: &mut World| {
    //         if let Some(mut sprite) = world.get_mut::<Sprite>(id) {
    //             if let Some(ref mut atlas) = sprite.texture_atlas.as_mut() {
    //                 atlas.index = sprite_index;
    //             } else {
    //                 warn!("No texture atlas for sprite {id}");
    //             }
    //         } else {
    //             warn!("No sprite {id}");
    //         }
    //     });
    // }

    #[cfg(feature = "level")]
    /// Get properties
    pub fn props(&self, id: Entity) -> Result<tiled::Properties, Error> {
        self.tiled.props(id)
    }

    pub fn pal_map(&mut self, original_to_new: Option<(usize, usize)>, mode: Option<PalModify>) {
        let mode = mode.unwrap_or_default();
        assert!(matches!(mode, PalModify::Following));
        if let Some((old, new)) = original_to_new {
            self.state.pal_map.remap(old, new);
        } else {
            // Reset the pal_map.
            self.state.pal_map.reset();
        }
    }

    /// Return the number of colors in the current palette.
    pub fn paln(&self, palette_index: Option<usize>) -> Result<usize, Error> {
        self.palette(palette_index).map(|pal| pal.data.len())
    }

    pub fn palt(&mut self, color_index: Option<usize>, transparent: Option<bool>) {
        if let Some(color_index) = color_index {
            self.state
                .pal_map
                .transparency
                .set(color_index, transparent.unwrap_or(false));
        } else {
            // Reset the pal_map.
            self.state.pal_map.reset_transparency();
        }
    }

    pub fn color(&mut self, color: Option<impl Into<PColor>>) -> Result<PColor, Error> {
        let last_color = self.state.draw_state.pen;
        if let Some(color) = color.map(|x| x.into()) {
            if let PColor::Palette(n) = color {
                // Check that it's within the palette.
                if n >= self.palette(None)?.data.len() {
                    return Err(Error::NoSuch("palette color index".into()));
                }
            }
            self.state.draw_state.pen = color;
        }
        Ok(last_color)
    }

    pub fn cursor(&mut self, pos: Option<Vec2>, color: Option<PColor>) -> (Vec2, PColor) {
        let last_pos = self.state.draw_state.print_cursor;
        let last_color = self.state.draw_state.pen;
        if let Some(pos) = pos.map(|p| self.state.draw_state.apply_camera_delta(p)) {
            self.state.draw_state.print_cursor = pos;
        }
        if let Some(color) = color {
            self.state.draw_state.pen = color;
        }
        (last_pos, last_color)
    }

    pub fn fillp(&mut self, pattern: Option<u16>) -> u16 {
        let last: u16 = self
            .state
            .draw_state
            .fill_pat
            .map(|x| x.into())
            .unwrap_or(0);
        if let Some(pattern) = pattern {
            if pattern == 0 {
                self.state.draw_state.fill_pat = None;
            } else {
                self.state.draw_state.fill_pat = Some(pattern.into());
            }
        }
        last
    }

    pub fn poke(&mut self, addr: usize, value: u8) -> Result<(), Error> {
        match addr {
            0x5f2d => {
                self.key_input.enabled = value != 0;
            }
            _ => Err(Error::UnsupportedPoke(addr))?,
        }
        Ok(())
    }

    pub fn peek(&mut self, addr: usize) -> Result<u8, Error> {
        Err(Error::UnsupportedPeek(addr))
    }

    #[cfg(feature = "scripting")]
    pub fn stat(&mut self, n: u8, value: Option<u8>) -> Result<ScriptValue, Error> {
        match n {
            30 => Ok(ScriptValue::Bool(!self.key_input.buffer.is_empty())),
            31 => self.key_input.pop().map(|string_maybe| {
                string_maybe
                    .map(ScriptValue::String)
                    .unwrap_or(ScriptValue::Unit)
            }),
            32 => Ok(ScriptValue::Float(self.mouse_input.position.x as f64)),
            33 => Ok(ScriptValue::Float(
                negate_y(self.mouse_input.position.y) as f64
            )),
            34 => Ok(ScriptValue::Integer(self.mouse_input.buttons as i64)),
            _ => Err(Error::UnsupportedStat(n))?,
        }
    }

    /// Return the size of the canvas
    ///
    /// This is not the window dimensions, which are physical pixels. Instead it
    /// is the number of "logical" pixels, which may be comprised of many
    /// physical pixels.
    pub fn canvas_size(&self) -> UVec2 {
        self.canvas.size
    }

    pub(crate) fn get_color(&self, c: impl Into<N9Color>) -> Result<Color, Error> {
        match c.into().into_pcolor(&self.state.draw_state.pen) {
            PColor::Palette(n) => self.palette(None)?.get_color(n).map(|c| c.into()),
            PColor::Color(c) => Ok(c.into()),
        }
    }
}

#[cfg(feature = "fixed")]
mod fixed {
    use fixed::types::extra::U16;
    use fixed::FixedI32;
    impl super::Pico8<'_, '_> {
        pub fn shl(a: f32, b: u8) -> f32 {
            let a = FixedI32::<U16>::from_num(a);
            let c = a << b;
            c.to_num()
        }

        pub fn shr(a: f32, b: u8) -> f32 {
            let a = FixedI32::<U16>::from_num(a);
            let c = a >> b;
            c.to_num()
        }

        pub fn lshr(a: f32, b: u8) -> f32 {
            let c: u32 = FixedI32::<U16>::from_num(a).to_bits() as u32;
            let d = c >> b;
            FixedI32::<U16>::from_bits(d as i32).to_num()
        }

        pub fn rotr(a: f32, b: u8) -> f32 {
            let c: u32 = FixedI32::<U16>::from_num(a).to_bits() as u32;
            let d = (c << (32 - b)) | (c >> b);
            FixedI32::<U16>::from_bits(d as i32).to_num()
        }

        pub fn rotl(a: f32, b: u8) -> f32 {
            let c: u32 = FixedI32::<U16>::from_num(a).to_bits() as u32;
            let d = (c << b) | (c >> (32 - b));
            FixedI32::<U16>::from_bits(d as i32).to_num()
        }
    }
}

#[derive(Default, Debug, Clone)]
pub enum PalModify {
    #[default]
    Following,
    Present,
    Secondary,
}

// XXX: Dump this after refactor.
impl FromWorld for Pico8State {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Pico8State {
            palette: 0,
            // palettes: vec![Palette::from_slice(&crate::pico8::PALETTE)].into(),
            // palettes: vec![].into(),
            //     handle: asset_server.load_with_settings(PICO8_PALETTE, pixel_art_settings),
            //     row: 0,
            // },
            pal_map: PalMap::default(),
            draw_state: {
                let mut draw_state = DrawState::default();
                // Need to set defaults somewhere.
                // draw_state.pen = PColor::Palette(6);
                draw_state.pen = PColor::Palette(1);
                draw_state
            },
        }
    }
}

impl FromWorld for Pico8Asset {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Pico8Asset {
            #[cfg(feature = "scripting")]
            code: None,
            palettes: vec![Palette::from_slice(&crate::pico8::PALETTE)].into(),
            // palettes: vec![].into(),
            //     handle: asset_server.load_with_settings(PICO8_PALETTE, pixel_art_settings),
            //     row: 0,
            // },
            border: asset_server.load_with_settings(PICO8_BORDER, pixel_art_settings),
            font: vec![N9Font {
                handle: asset_server.load(PICO8_FONT),
            }]
            .into(),
            // draw_state: {
            //     let mut draw_state = DrawState::default();
            //     draw_state.pen = PColor::Palette(6);
            //     draw_state
            // },
            audio_banks: Vec::new().into(),
            sprite_sheets: Vec::new().into(),
            maps: Vec::new().into(),
        }
    }
}

impl Pico8Asset {
    pub(crate) fn get_color(&self, c: PColor, palette_index: usize) -> Result<Color, Error> {
        match c {
            PColor::Palette(n) => self.palettes[palette_index].get_color(n).map(|c| c.into()),
            PColor::Color(c) => Ok(c.into()),
        }
    }
}

pub(crate) fn plugin(app: &mut App) {
    embedded_asset!(app, "pico-8-palette.png");
    embedded_asset!(app, "rect-border.png");
    embedded_asset!(app, "pico-8.ttf");
    app.register_type::<Pico8Asset>()
        .register_type::<Pico8State>()
        .register_type::<N9Font>()
        .register_type::<Palette>()
        .register_type::<SpriteSheet>()
        .init_asset::<Pico8Asset>()
        .init_resource::<Pico8State>()
        .init_resource::<PlayerInputs>()
        .add_observer(
            |trigger: Trigger<UpdateCameraPos>,
             camera: Single<&mut Transform, With<Nano9Camera>>| {
                let pos = trigger.event();
                let mut camera = camera.into_inner();
                camera.translation.x = pos.0.x;
                camera.translation.y = negate_y(pos.0.y);
            },
        );
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_suffix_match() {
        let s = "a\\0";
        assert_eq!(s.len(), 3);
        assert!(s.ends_with("\\0"));
    }

    #[test]
    fn test_buttons() {
        let mut b = Buttons::default();
        assert!(!b.btn(None).unwrap());
        assert!(!b.btnp(None).unwrap());
        b.curr.set(0, true);
        assert!(b.btn(None).unwrap());
        assert!(b.btnp(None).unwrap());
        b.last.set(1, true);
        assert!(b.btn(None).unwrap());
        assert!(b.btnp(None).unwrap());
        b.curr.set(1, true);
        assert!(b.btn(None).unwrap());
        assert!(b.btnp(None).unwrap());
        b.last = b.curr;
        assert!(b.btn(None).unwrap());
        assert!(!b.btnp(None).unwrap());
        b.curr.set(0, false);
        b.curr.set(1, false);
        b.last.set(1, false);
        assert!(!b.btn(None).unwrap());
        assert!(!b.btnp(None).unwrap());
    }

    #[cfg(feature = "fixed")]
    mod fixed {
        use super::*;
        #[test]
        fn test_shr() {
            assert_eq!(0.5, Pico8::shr(1.0, 1));
            assert_eq!(-0.5, Pico8::shr(-1.0, 1));
        }

        #[test]
        fn test_lshr() {
            assert_eq!(0.5, Pico8::lshr(1.0, 1));
            assert_eq!(32767.5, Pico8::lshr(-1.0, 1));
            assert_eq!(8191.875, Pico8::lshr(-1.0, 3));
        }

        #[test]
        fn test_shl() {
            assert_eq!(2.0, Pico8::shl(1.0, 1));
        }

        #[test]
        fn test_rotr() {
            assert_eq!(Pico8::rotr(64.0, 3), 8.0);
            assert_eq!(Pico8::rotr(1.0, 3), 0.125);
            assert_eq!(Pico8::rotr(-4096.0, 12), 15.0);
        }

        #[test]
        fn test_rotl() {
            assert_eq!(Pico8::rotl(8.0, 3), 64.0);
            assert_eq!(Pico8::rotl(0.125, 3), 1.0);
            assert_eq!(Pico8::rotl(-4096.0, 12), 0.05859375);
        }
    }
}

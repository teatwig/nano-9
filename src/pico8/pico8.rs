use bevy::{
    asset::embedded_asset,
    audio::PlaybackMode,
    ecs::system::SystemParam,
    image::{ImageLoaderSettings, ImageSampler, TextureAccessError},
    input::{keyboard::Key,
            gamepad::GamepadConnectionEvent},
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    sprite::Anchor,
};
use tiny_skia::{self, FillRule, Paint, PathBuilder, Pixmap, Stroke};

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
        image::pixel_art_settings,
        keyboard::KeyInput,
        mouse::MouseInput,
        audio::{Sfx, SfxChannels, AudioBank, AudioCommand, SfxDest},
        rand::Rand8,
        Cart, ClearEvent, Clearable, Gfx, GfxHandles, LoadCart, Map, PalMap, PALETTE, Palette,
    },
    DrawState, FillColor, N9Canvas, N9Color, Nano9Camera, PColor,
};

use std::{
    collections::VecDeque,
    any::TypeId,
    borrow::Cow,
    f32::consts::PI,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

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
    pub height: Option<f32>,
}

/// Pico8State's state.
#[derive(Resource, Clone, Asset, Debug, Reflect)]
#[reflect(Resource)]
pub struct Pico8State {
    pub code: Handle<ScriptAsset>,
    pub(crate) palettes: Cursor<Palette>,
    #[reflect(ignore)]
    pub(crate) pal_map: PalMap,
    // XXX: rename to gfx_images?
    pub(crate) border: Handle<Image>,
    pub(crate) sprite_sheets: Cursor<SpriteSheet>,
    pub(crate) maps: Cursor<Map>,
    pub(crate) font: Cursor<N9Font>,
    pub(crate) draw_state: DrawState,
    pub(crate) audio_banks: Cursor<AudioBank>,
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

impl FromScript for Spr {
    type This<'w> = Self;
    fn from_script(
        value: ScriptValue,
        _world: WorldAccessGuard<'_>,
    ) -> Result<Self::This<'_>, InteropError> {
        match value {
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

impl From<(usize, usize)> for Spr {
    fn from((sprite, sheet): (usize, usize)) -> Self {
        Spr::From { sprite, sheet }
    }
}

#[derive(Debug, Clone, Reflect)]
pub enum SprAsset {
    Gfx(Handle<Gfx>),
    Image(Handle<Image>),
}

#[derive(Debug, Clone, Reflect)]
pub struct SpriteSheet {
    pub handle: SprAsset,
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

impl TypedThrough for PropBy {
    fn through_type_info() -> ThroughTypeInfo {
        ThroughTypeInfo::TypeInfo(<PropBy as bevy::reflect::Typed>::type_info())
    }
}

fn script_value_to_f32(value: &ScriptValue) -> Option<f32> {
    match value {
        ScriptValue::Float(f) => Some(*f as f32),
        ScriptValue::Integer(i) => Some(*i as f32),
        _ => None,
    }
}

impl FromScript for PropBy {
    type This<'w> = Self;
    fn from_script(
        value: ScriptValue,
        _world: WorldAccessGuard<'_>,
    ) -> Result<Self::This<'_>, InteropError> {
        match value {
            ScriptValue::String(n) => Ok(PropBy::Name(n)),
            ScriptValue::List(l) => {
                let x = l.first().and_then(script_value_to_f32).unwrap_or(0.0);
                let y = l.get(1).and_then(script_value_to_f32).unwrap_or(0.0);
                Ok(PropBy::Pos(Vec2::new(x, y)))
            }
            ScriptValue::Map(v) => {
                let x = v.get("x").and_then(script_value_to_f32).unwrap_or(0.0);
                let y = v.get("y").and_then(script_value_to_f32).unwrap_or(0.0);
                let w = v.get("width").and_then(script_value_to_f32);
                let h = v.get("height").and_then(script_value_to_f32);
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
    #[allow(dead_code)]
    pub fn load_cart(&mut self, cart: Handle<Cart>) {
        self.commands.spawn(LoadCart(cart));
    }

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
        let sheet = &self.state.sprite_sheets.inner[sheet_index];
        let sprite = Sprite {
            image: match &sheet.handle {
                SprAsset::Image(handle) => handle.clone(),
                SprAsset::Gfx(handle) => self.gfx_handles.get_or_create(
                    &self.state.palettes,
                    &self.state.pal_map,
                    None,
                    handle,
                    &self.gfxs,
                    &mut self.images,
                )?,
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
            Spr::Cur { sprite } => (&self.state.sprite_sheets, sprite),
            Spr::From { sheet, sprite } => (&self.state.sprite_sheets.inner[sheet], sprite),
            Spr::Set { sheet } => {
                self.state.sprite_sheets.pos = sheet;
                return Ok(Entity::PLACEHOLDER);
            }
        };
        let image = match &sprites.handle {
            SprAsset::Image(handle) => handle.clone(),
            SprAsset::Gfx(handle) => self.gfx_handles.get_or_create(
                &self.state.palettes,
                &self.state.pal_map,
                None,
                handle,
                &self.gfxs,
                &mut self.images,
            )?,
        };
        assert!(image.is_strong());
        let mut sprite = {
            let atlas = TextureAtlas {
                layout: sprites.layout.clone(),
                index,
            };
            Sprite {
                image,
                anchor: Anchor::TopLeft,
                texture_atlas: Some(atlas),
                rect: size.map(|v| Rect {
                    min: Vec2::ZERO,
                    max: sprites.sprite_size.as_vec2() * v,
                }),
                flip_x: flip.x,
                flip_y: flip.y,
                ..default()
            }
        };
        let clearable = Clearable::default();
        let mut transform = Transform::from_xyz(x, negate_y(y), clearable.suggest_z());
        if let Some(turns) = turns {
            let pixel_size = sprites.sprite_size.as_vec2() * size.unwrap_or(Vec2::ONE) / 2.0;
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

    // XXX: Should this be here? It's not a Pico8 API.
    pub(crate) fn get_color(&self, c: impl Into<N9Color>) -> Result<Color, Error> {
        match c.into() {
            N9Color::Pen => match self.state.draw_state.pen {
                PColor::Palette(n) => {
                    self.state.palettes.get_color(n).map(|c| c.into())
                    // let pal = self
                    //     .images
                    //     .get(&self.state.palettes.handle)
                    //     .ok_or(Error::NoAsset("palette".into()))?;

                    // // Strangely. It's not a 1d texture.
                    // Ok(pal.get_color_at(n as u32, self.state.palettes.row)?)
                }
                PColor::Color(c) => Ok(c.into()),
            },
            N9Color::Palette(n) => {
                self.state.palettes.get_color(n).map(|c| c.into())
                // let pal = self
                //     .images
                //     .get(&self.state.palettes.handle)
                //     .ok_or(Error::NoAsset("palette".into()))?;

                // // Strangely. It's not a 1d texture.
                // Ok(pal.get_color_at(n as u32, self.state.palettes.row)?)
            }
            N9Color::Color(c) => Ok(c.into()),
        }
    }

    // cls([n])
    pub fn cls(&mut self, color: Option<N9Color>) -> Result<(), Error> {
        let c = self.get_color(color.unwrap_or(Color::BLACK.into()))?;
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

    pub fn pset(&mut self, pos: UVec2, color: Option<N9Color>) -> Result<(), Error> {
        let c = self.get_color(color.unwrap_or(N9Color::Pen))?;
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
        let sheet_index = sheet_index.unwrap_or(0);
        let sheet = &self.state.sprite_sheets.inner[sheet_index];
        match &sheet.handle {
            SprAsset::Gfx(handle) => {
                let gfx = self
                    .gfxs
                    .get_mut(handle)
                    .ok_or(Error::NoSuch("Gfx".into()))?;
                gfx.set(
                    pos.x as usize,
                    pos.y as usize,
                    match color {
                        N9Color::Palette(n) => Ok(n as u8),
                        N9Color::Pen => match self.state.draw_state.pen {
                            PColor::Palette(n) => Ok(n as u8),
                            PColor::Color(_) => Err(Error::InvalidArgument(
                                "Cannot write pen `Color` to Gfx asset".into(),
                            )),
                        },
                        N9Color::Color(_) => Err(Error::InvalidArgument(
                            "Cannot write arg `Color` to Gfx asset".into(),
                        )),
                    }?,
                );
            }
            SprAsset::Image(handle) => {
                let c = self.get_color(color)?;
                let image = self
                    .images
                    .get_mut(handle)
                    .ok_or(Error::NoAsset("canvas".into()))?;
                image.set_color_at(pos.x, pos.y, c)?;
            }
        }
        Ok(())
    }

    pub fn sget(&mut self, pos: UVec2, sheet_index: Option<usize>) -> Result<Option<PColor>, Error> {
        let sheet_index = sheet_index.unwrap_or(0);
        let sheet = &self.state.sprite_sheets.inner[sheet_index];
        Ok(match &sheet.handle {
            SprAsset::Gfx(handle) => {
                let gfx = self.gfxs.get(handle).ok_or(Error::NoSuch("Gfx".into()))?;
                gfx.get(pos.x as usize, pos.y as usize).map(|i| PColor::Palette(i as usize))
            }
            SprAsset::Image(handle) => {
                let image = self
                    .images
                    .get_mut(handle)
                    .ok_or(Error::NoAsset("canvas".into()))?;
                Some(PColor::Color(image.get_color_at(pos.x, pos.y)?.into()))
            }
        })
    }

    pub fn rectfill(
        &mut self,
        upper_left: Vec2,
        lower_right: Vec2,
        color: Option<FillColor>,
    ) -> Result<Entity, Error> {
        let upper_left = self.state.draw_state.apply_camera_delta(upper_left);
        let lower_right = self.state.draw_state.apply_camera_delta(lower_right);
        let size = (lower_right - upper_left) + Vec2::ONE;
        let clearable = Clearable::default();
        let id = self
            .commands
            .spawn((
                Name::new("rectfill"),
                if let Some(fill_pat) = &self.state.draw_state.fill_pat {
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
                        image: self.images.add(
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
                                    let _ =
                                        c.write_color(&(*self.state.palettes).data, &self.state.pal_map, pixel_bytes);
                                }
                                Ok::<(), Error>(())
                            })?,
                        ),
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
                    image: self.state.border.clone(),
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

    pub fn map(
        &mut self,
        map_pos: UVec2,
        mut screen_start: Vec2,
        size: UVec2,
        mask: Option<u8>,
        map_index: Option<usize>,
    ) -> Result<Entity, Error> {
        screen_start = self.state.draw_state.apply_camera_delta(screen_start);
        let map_index = map_index.unwrap_or(0);
        if cfg!(feature = "negate-y") {
            screen_start.y = -screen_start.y;
        }
        match self
            .state
            .maps
            .inner
            .get(map_index)
            .ok_or(Error::InvalidArgument("no map".into()))?
        {
            Map::P8(ref map) => map.map(
                map_pos,
                screen_start,
                size,
                mask,
                &self.state.sprite_sheets.inner,
                &mut self.commands,
                |handle| {
                    self.gfx_handles.get_or_create(
                        &self.state.palettes,
                        &self.state.pal_map,
                        None,
                        handle,
                        &self.gfxs,
                        &mut self.images,
                    )
                },
            ),
            #[cfg(feature = "level")]
            Map::Level(ref map) => Ok(map.map(screen_start, 0, &mut self.commands)),
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

    // print(text, [x,] [y,] [color])
    pub fn print(
        &mut self,
        text: impl AsRef<str>,
        pos: Option<Vec2>,
        color: Option<N9Color>,
    ) -> Result<f32, Error> {
        let mut text: &str = text.as_ref();
        let pos = pos.map(|p| self.state.draw_state.apply_camera_delta(p)).unwrap_or_else(|| {
            Vec2::new(
                self.state.draw_state.print_cursor.x,
                self.state.draw_state.print_cursor.y,
            )
        });
        let c = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let clearable = Clearable::default();
        let add_newline = if text.ends_with('\0') {
            text = &text[..text.len().saturating_sub(1)];
            false
        } else {
            true
        };
        // XXX: this is byte count, not char count.
        let len = text.len() as f32;
        let font_size = 5.0;
        // Empirically derived the char_width from the font size using these
        // good values for the Pico-8 font:
        //
        // (font_size, char_width)
        //  5, 4
        // 10, 8
        let char_width = font_size * 4.0 / 5.0;
        let z = clearable.suggest_z();
        self.commands
            .spawn((
                Name::new("print"),
                Transform::from_xyz(pos.x, negate_y(pos.y), z),
                Text2d::new(text),
                Visibility::default(),
                TextFont {
                    font: self.state.font.handle.clone(),
                    font_smoothing: bevy::text::FontSmoothing::None,
                    font_size,
                },
                Anchor::TopLeft,
                clearable,
            ));
        if add_newline {
            self.state.draw_state.print_cursor.x = pos.x;
            self.state.draw_state.print_cursor.y = pos.y + font_size + 1.0;
        } else {
            self.state.draw_state.print_cursor.x = pos.x + char_width * len;
        }
        self.state.draw_state.mark_drawn();
        Ok(pos.x + len * char_width)
    }

    pub fn exit(&mut self, error: Option<u8>) {
        self.commands.send_event(match error {
            Some(n) => {
                std::num::NonZero::new(n)
                    .map(AppExit::Error).unwrap_or(AppExit::Success)
            },
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
                    self.commands
                        .queue(AudioCommand::Stop(SfxDest::Channel(chan), Some(PlaybackMode::Remove)));
                } else {
                    self.commands.queue(AudioCommand::Stop(SfxDest::All, Some(PlaybackMode::Remove)));
                }
            }
            SfxCommand::Play(n) => {
                let sfx = self.state.audio_banks.inner[bank as usize]
                    .get(n as usize)
                    .ok_or(Error::NoAsset(format!("sfx {n}").into()))?
                    .clone();

                if let Some(chan) = channel {
                    // let chan = self.sfx_channels[chan as usize];
                    self.commands
                        .queue(AudioCommand::Play(sfx, SfxDest::Channel(chan), PlaybackSettings::REMOVE));
                } else {
                    self.commands.queue(AudioCommand::Play(sfx, SfxDest::Any, PlaybackSettings::REMOVE));
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
                    self.commands.queue(AudioCommand::Stop(SfxDest::All, Some(PlaybackMode::Loop)));
                // }
            }
            SfxCommand::Play(n) => {
                let sfx = self.state.audio_banks.inner.get(bank as usize)
                                                      .ok_or(Error::NoSuch(format!("audio bank {bank}").into()))?
                    .get(n as usize)
                    .ok_or(Error::NoAsset(format!("music {n}").into()))?
                    .clone();

                if let Some(mask) = channel_mask {
                    self.commands
                        .queue(AudioCommand::Play(sfx, SfxDest::ChannelMask(mask), PlaybackSettings::LOOP));
                } else {
                    self.commands.queue(AudioCommand::Play(sfx, SfxDest::Any, PlaybackSettings::LOOP));
                }
            }
        }
        Ok(())
    }

    pub fn fget(&self, index: Option<usize>, flag_index: Option<u8>) -> u8 {
        if index.is_none() {
            return 0;
        }
        let index = index.unwrap();
        let flags = &self.state.sprite_sheets.flags;
        if let Some(v) = flags.get(index) {
            match flag_index {
                Some(flag_index) => {
                    if v & (1 << flag_index) != 0 {
                        1
                    } else {
                        0
                    }
                }
                None => *v,
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
            0
        }
    }

    pub fn fset(&mut self, index: usize, flag_index: Option<u8>, value: u8) {
        let flags = &mut self.state.sprite_sheets.flags;
        match flag_index {
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
        }
    }

    #[cfg(feature = "level")]
    /// Get properties
    pub fn mgetp(
        &self,
        prop_by: PropBy,
        map_index: Option<usize>,
        layer_index: Option<usize>,
    ) -> Option<tiled::Properties> {
        let map: &Map = self.state.maps.get(map_index).expect("No such map");
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
        let map: &Map = self.state.maps.get(map_index).expect("No such map");
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
        let map: &mut Map = self.state.maps.get_mut(map_index).expect("No such map");
        match *map {
            Map::P8(ref mut map) => map
                .get_mut((pos.x as u32 + pos.y as u32 * MAP_COLUMNS) as usize)
                .map(|value| *value = sprite_index as u8)
                .ok_or(Error::NoSuch("map entry".into())),
            #[cfg(feature = "level")]
            Map::Level(ref mut map) => {
                self.tiled
                    .mset(map, pos, sprite_index, map_index, layer_index)
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

    pub fn color(&mut self, color: Option<PColor>) -> PColor {
        let last_color = self.state.draw_state.pen;
        if let Some(color) = color {
            self.state.draw_state.pen = color;
        }
        last_color
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
            0x5f2d => { self.key_input.enabled = value != 0; },
            _ => Err(Error::UnsupportedPoke(addr))?,
        }
        Ok(())
    }

    pub fn peek(&mut self, addr: usize) -> Result<u8, Error> {
        match addr {
            // 0x5f2d => self.state.peek_keycodes = if value == 0 { false } else { true },
            _ => Err(Error::UnsupportedPeek(addr)),
        }
    }

    pub fn stat(&mut self, n: u8, value: Option<u8>) -> Result<ScriptValue, Error> {
        match n {
            30 => Ok(ScriptValue::Bool(!self.key_input.buffer.is_empty())),
            31 => self.key_input.pop().map(|string_maybe| string_maybe.map(ScriptValue::String).unwrap_or(ScriptValue::Unit)),
            32 => Ok(ScriptValue::Float(self.mouse_input.position.x as f64)),
            33 => Ok(ScriptValue::Float(negate_y(self.mouse_input.position.y) as f64)),
            34 => Ok(ScriptValue::Integer(self.mouse_input.buttons as i64)),
            _ => Err(Error::UnsupportedStat(n))?,
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

impl FromWorld for Pico8State {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Pico8State {
            palettes: vec![Palette::from_slice(&PALETTE)].into(),
            //     handle: asset_server.load_with_settings(PICO8_PALETTE, pixel_art_settings),
            //     row: 0,
            // },
            pal_map: PalMap::default(),
            border: asset_server.load_with_settings(PICO8_BORDER, pixel_art_settings),
            code: Handle::<ScriptAsset>::default(),
            font: vec![N9Font {
                handle: asset_server.load(PICO8_FONT),
                height: Some(7.0),
            }]
            .into(),
            draw_state: DrawState::default(),
            audio_banks: Vec::new().into(),
            sprite_sheets: Vec::new().into(),
            maps: Vec::new().into(),
        }
    }
}

pub(crate) fn plugin(app: &mut App) {
    embedded_asset!(app, "pico-8-palette.png");
    embedded_asset!(app, "rect-border.png");
    embedded_asset!(app, "pico-8.ttf");
    app.register_type::<Pico8State>()
        .register_type::<N9Font>()
        .register_type::<Palette>()
        .register_type::<SpriteSheet>()
        .init_asset::<Pico8State>()
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
}

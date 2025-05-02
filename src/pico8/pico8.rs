use bevy::{
    asset::embedded_asset,
    ecs::system::SystemParam,
    image::{ImageLoaderSettings, ImageSampler, TextureAccessError},
    input::gamepad::GamepadConnectionEvent,
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
        PALETTE,
        audio::{Sfx, SfxChannels},
        rand::Rand8,
        Cart, ClearEvent, Clearable, Gfx, LoadCart, Map, PalMap, GfxHandles,
    },
    DrawState, N9Canvas, N9Color, Nano9Camera, PColor, FillColor,
};

use std::{
    any::TypeId,
    borrow::Cow,
    collections::HashMap,
    f32::consts::PI,
    hash::Hasher,
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

#[derive(Clone, Debug, Deref, DerefMut, Reflect)]
pub struct AudioBank(pub Vec<Audio>);

#[derive(Debug, Clone, Reflect)]
pub enum Audio {
    Sfx(Handle<Sfx>),
    AudioSource(Handle<AudioSource>),
}

#[derive(Debug, Clone, Reflect)]
pub struct Palette {
    pub handle: Handle<Image>,
    /// Row count
    pub row: u32,
}

/// Pico8State's state.
#[derive(Resource, Clone, Asset, Debug, Reflect)]
#[reflect(Resource)]
pub struct Pico8State {
    pub code: Handle<ScriptAsset>,
    pub(crate) palette: Palette,
    #[reflect(ignore)]
    pub(crate) pal_map: PalMap,
    // XXX: rename to gfx_images?
    #[reflect(ignore)]
    pub(crate) gfx_handles: HashMap<(PalMap, Handle<Gfx>), Handle<Image>>,
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
    #[error("No such {0:?}")]
    NoSuch(Cow<'static, str>),
    #[error("No asset {0:?} loaded")]
    NoAsset(Cow<'static, str>),
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
}

impl From<Error> for LuaError {
    fn from(e: Error) -> Self {
        LuaError::RuntimeError(format!("pico8 error: {e}"))
    }
}

#[derive(SystemParam)]
pub struct Pico8<'w, 's> {
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
        let x = screen_pos.x;
        let y = screen_pos.y;
        let flip = flip.unwrap_or_default();
        let sheet_index = sheet_index.unwrap_or(0);
        let sheet = &self.state.sprite_sheets.inner[sheet_index];
        let sprite = Sprite {
            image: match &sheet.handle {
                SprAsset::Image(handle) => handle.clone(),
                SprAsset::Gfx(handle) => self.gfx_handles.get_or_create(
                    &self.state.pal_map,
                    None,
                    handle,
                    &self.gfxs,
                    &mut self.images,
                ),
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
                &self.state.pal_map,
                None,
                handle,
                &self.gfxs,
                &mut self.images,
            ),
        };
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
                    let pal = self
                        .images
                        .get(&self.state.palette.handle)
                        .ok_or(Error::NoAsset("palette".into()))?;

                    // Strangely. It's not a 1d texture.
                    Ok(pal.get_color_at(n as u32, self.state.palette.row)?)
                }
                PColor::Color(c) => Ok(c.into()),
            },
            N9Color::Palette(n) => {
                let pal = self
                    .images
                    .get(&self.state.palette.handle)
                    .ok_or(Error::NoAsset("palette".into()))?;

                // Strangely. It's not a 1d texture.
                Ok(pal.get_color_at(n as u32, self.state.palette.row)?)
            }
            N9Color::Color(c) => Ok(c.into()),
        }
    }

    // cls([n])
    pub fn cls(&mut self, color: Option<N9Color>) -> Result<(), Error> {
        let c = self.get_color(color.unwrap_or(Color::BLACK.into()))?;
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

    pub fn sget(&mut self, pos: UVec2, sheet_index: Option<usize>) -> Result<PColor, Error> {
        let sheet_index = sheet_index.unwrap_or(0);
        let sheet = &self.state.sprite_sheets.inner[sheet_index];
        Ok(match &sheet.handle {
            SprAsset::Gfx(handle) => {
                let gfx = self.gfxs.get(handle).ok_or(Error::NoSuch("Gfx".into()))?;
                PColor::Palette(gfx.get(pos.x as usize, pos.y as usize) as usize)
            }
            SprAsset::Image(handle) => {
                let image = self
                    .images
                    .get_mut(handle)
                    .ok_or(Error::NoAsset("canvas".into()))?;
                PColor::Color(image.get_color_at(pos.x, pos.y)?.into())
            }
        })
    }

    pub fn rectfill(
        &mut self,
        upper_left: Vec2,
        lower_right: Vec2,
        color: Option<FillColor>,
    ) -> Result<Entity, Error> {
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
                            fill_pat.to_image(8, 8, |bit, pixel_index, pixel_bytes| {
                            let c: Option<PColor> = if bit {
                                color.and_then(|x| x.on())
                            } else {
                                color.map(|x| x.off()).or(Some(self.state.draw_state.pen))
                            };
                            if let Some(c) = c {
                                // c.map(&self.state.pal_map).write_color(&PALETTE, pixel_bytes);
                                c.write_color(&PALETTE, &self.state.pal_map, pixel_bytes);
                            }
                            Ok::<(), Error>(())
                        })?
                        ),
                        custom_size: Some(size),
                        image_mode: SpriteImageMode::Tiled { tile_x: true, tile_y: true, stretch_value: 1.0 },
                        ..default()
                    }
                } else {
                    let c = self.get_color(color.map(|x| x.off().into()).unwrap_or(N9Color::Pen))?;
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
        Ok(id)
    }

    pub fn rect(
        &mut self,
        upper_left: Vec2,
        lower_right: Vec2,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
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
        const CHAR_WIDTH: f32 = 4.0;
        const NEWLINE_HEIGHT: f32 = 6.0;
        let mut text: &str = text.as_ref();
        // warn!("PRINTING {}", &text);
        // info!("print {:?} start, {:?}", &text, &self.state.draw_state.print_cursor);
        let pos = pos.unwrap_or_else(|| {
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
        let len = text.len() as f32;
        let z = clearable.suggest_z();
        self.commands
            .spawn((
                Name::new("print"),
                Transform::from_xyz(pos.x, negate_y(pos.y), z),
                Visibility::default(),
                clearable,
            ))
            .with_children(|builder| {
                let mut y = 0.0;
                for line in text.lines() {
                    // Our font has a different height than we want. It's one pixel
                    // higher. So we can't let bevy render it one go. Bummer.
                    builder.spawn((
                        Text2d::new(line),
                        Transform::from_xyz(0.0, negate_y(y), z),
                        TextColor(c),
                        TextFont {
                            font: self.state.font.handle.clone(),
                            font_smoothing: bevy::text::FontSmoothing::None,
                            font_size: 6.0,
                        },
                        // Anchor::TopLeft is (-0.5, 0.5).
                        Anchor::Custom(Vec2::new(-0.5, 0.3)),
                    ));
                    y += NEWLINE_HEIGHT;
                }
            });
        if add_newline {
            self.state.draw_state.print_cursor.x = pos.x;
            self.state.draw_state.print_cursor.y = pos.y + NEWLINE_HEIGHT;
        } else {
            self.state.draw_state.print_cursor.x = pos.x + CHAR_WIDTH * len;
        }
        // info!("print end, {:?}", &self.state.draw_state.print_cursor);
        // XXX: Need the font width somewhere.
        Ok(pos.x + len * CHAR_WIDTH)
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
                    let chan = self.sfx_channels[chan as usize];
                    self.commands
                        .queue(AudioCommand::Release(SfxDest::Channel(chan)));
                } else {
                    self.commands.queue(AudioCommand::Release(SfxDest::Any));
                }
            }
            SfxCommand::Stop => {
                if let Some(chan) = channel {
                    let chan = self.sfx_channels[chan as usize];
                    self.commands
                        .queue(AudioCommand::Stop(SfxDest::Channel(chan)));
                } else {
                    self.commands.queue(AudioCommand::Stop(SfxDest::All));
                }
            }
            SfxCommand::Play(n) => {
                let sfx = self.state.audio_banks.inner[bank as usize]
                    .get(n as usize)
                    .ok_or(Error::NoAsset(format!("sfx {n}").into()))?
                    .clone();

                if let Some(chan) = channel {
                    let chan = self.sfx_channels[chan as usize];
                    self.commands
                        .queue(AudioCommand::Play(sfx, SfxDest::Channel(chan)));
                } else {
                    self.commands.queue(AudioCommand::Play(sfx, SfxDest::Any));
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
            self.commands.trigger(UpdateCameraPos(pos));
            last
        } else {
            self.state.draw_state.camera_position
        }
    }

    pub fn line(&mut self, a: IVec2, b: IVec2, color: Option<N9Color>) -> Result<Entity, Error> {
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

        let image = Image::new(
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
        Ok(id)
    }

    pub fn circ(
        &mut self,
        pos: IVec2,
        r: impl Into<UVec2>,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
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

        let image = Image::new(
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
        Ok(id)
    }

    pub fn ovalfill(
        &mut self,
        upper_left: IVec2,
        lower_right: IVec2,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
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

        let image = Image::new(
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
        Ok(id)
    }

    pub fn oval(
        &mut self,
        upper_left: IVec2,
        lower_right: IVec2,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
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

        let image = Image::new(
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
        if let Some(pos) = pos {
            self.state.draw_state.print_cursor = pos;
        }
        if let Some(color) = color {
            self.state.draw_state.pen = color;
        }
        (last_pos, last_color)
    }

    pub fn fillp(&mut self, pattern: Option<u16>) -> u16 {
        let last: u16 = self.state.draw_state.fill_pat.map(|x| x.into()).unwrap_or(0);
        if let Some(pattern) = pattern {
            if pattern == 0 {
                self.state.draw_state.fill_pat = None;
            } else {
                self.state.draw_state.fill_pat = Some(pattern.into());
            }
        }
        last
    }
}

#[derive(Default, Debug, Clone)]
pub enum PalModify {
    #[default]
    Following,
    Present,
    Secondary,
}

enum SfxDest {
    Any,
    All,
    Channel(Entity),
}

enum AudioCommand {
    Stop(SfxDest),
    Play(Audio, SfxDest),
    Release(SfxDest),
}

#[derive(Component)]
struct SfxRelease(Arc<AtomicBool>);

impl Command for AudioCommand {
    fn apply(self, world: &mut World) {
        match self {
            AudioCommand::Stop(sfx_channel) => {
                match sfx_channel {
                    SfxDest::All => {
                        // TODO: Consider using smallvec for channels.
                        let channels: Vec<Entity> = (*world.resource::<SfxChannels>()).clone();
                        for chan in channels {
                            if let Some(ref mut sink) = world.get_mut::<AudioSink>(chan) {
                                sink.stop();
                            }
                        }
                    }
                    SfxDest::Channel(chan) => {
                        if let Some(ref mut sink) = world.get_mut::<AudioSink>(chan) {
                            sink.stop();
                        }
                    }
                    SfxDest::Any => {
                        warn!("Cannot stop 'any' channels.");
                    }
                }
            }
            AudioCommand::Release(sfx_channel) => match sfx_channel {
                SfxDest::Channel(channel) => {
                    if let Some(sfx_release) = world.get::<SfxRelease>(channel) {
                        sfx_release.0.store(true, Ordering::Relaxed);
                    } else {
                        warn!("Released a channel that did not have a sfx loop.");
                    }
                }
                SfxDest::Any => {}
                SfxDest::All => {}
            },
            AudioCommand::Play(audio, sfx_channel) => {
                match sfx_channel {
                    SfxDest::Any => {
                        if let Some(available_channel) = world
                            .resource::<SfxChannels>()
                            .iter()
                            .find(|id| {
                                world
                                    .get::<AudioSink>(**id)
                                    .map(|s| s.is_paused() || s.empty())
                                    .unwrap_or(true)
                            })
                            .copied()
                        {
                            match audio {
                                Audio::Sfx(sfx) => {
                                    let (sfx, release) = Sfx::get_stoppable_handle(sfx, world);
                                    let mut commands = world.commands();
                                    if let Some(release) = release {
                                        commands
                                            .entity(available_channel)
                                            .insert(SfxRelease(release));
                                    }
                                    commands
                                        .entity(available_channel)
                                        .insert((AudioPlayer(sfx), PlaybackSettings::REMOVE));
                                }
                                Audio::AudioSource(_source) => {
                                    todo!();
                                }
                            }
                        } else {
                            // The channels may be busy. If we log it, it can be
                            // noisy in the log despite it not having much of an
                            // effect to the game, so we're not going to log it.

                            // warn!("Channels busy.");
                        }
                    }
                    SfxDest::Channel(chan) => {
                        let mut commands = world.commands();
                        match audio {
                            Audio::Sfx(sfx) => {
                                commands
                                    .entity(chan)
                                    .insert((AudioPlayer(sfx.clone()), PlaybackSettings::REMOVE));
                            }
                            Audio::AudioSource(_source) => {
                                todo!()
                            }
                        }
                    }
                    SfxDest::All => {
                        warn!("Cannot play on all channels.");
                    }
                }
            }
        }
    }
}

impl FromWorld for Pico8State {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        let pixel_art_settings = |settings: &mut ImageLoaderSettings| {
            // Use `nearest` image sampling to preserve the pixel art style.
            settings.sampler = ImageSampler::nearest();
        };

        Pico8State {
            gfx_handles: HashMap::default(),
            palette: Palette {
                handle: asset_server.load_with_settings(PICO8_PALETTE, pixel_art_settings),
                row: 0,
            },
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
        .register_type::<Audio>()
        .register_type::<AudioBank>()
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

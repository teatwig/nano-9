use bevy::{
    ecs::system::{SystemParam, SystemState},
    image::{ImageLoaderSettings, ImageSampler, TextureAccessError},
    prelude::*,
    render::{
        camera::ScalingMode,
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    sprite::Anchor,
};
use tiny_skia::{self, FillRule, Paint, PathBuilder, Pixmap, Stroke};

use bevy_mod_scripting::{
    core::{
        asset::{AssetPathToLanguageMapper, Language, ScriptAssetSettings, ScriptAsset},
        bindings::{
            WorldAccessGuard,
            access_map::ReflectAccessId,
            function::{
                from::{Val, Ref, Mut, FromScript},
                into_ref::IntoScriptRef,
                namespace::{GlobalNamespace, NamespaceBuilder},
                script_function::FunctionCallContext,
            },
            script_value::ScriptValue,
            ReflectReference,
        },
        error::InteropError,
    },
    lua::mlua::prelude::LuaError,
};
use rand::{seq::SliceRandom, Rng};

use bevy_ecs_tilemap::prelude::*;

use crate::{
    cursor::Cursor,
    pico8::{
        audio::{Sfx, SfxChannels},
        Cart, ClearEvent, Clearable, LoadCart,
    },
    DrawState, DropPolicy, N9Color, N9Entity, Nano9Camera, N9Canvas,
};

#[cfg(feature = "level")]
use crate::level;
use std::{
    borrow::Cow,
    path::Path,
    any::TypeId,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub const PICO8_PALETTE: &str = "images/pico-8-palette.png";
pub const PICO8_SPRITES: &str = "images/pooh-book-sprites.png";
pub const PICO8_BORDER: &str = "images/rect-border.png";
pub const PICO8_FONT: &str = "fonts/pico-8.ttf";
pub const MAP_COLUMNS: u32 = 128;
pub const PICO8_SPRITE_SIZE: UVec2 = UVec2::new(8, 8);
pub const PICO8_TILE_COUNT: UVec2 = UVec2::new(16, 16);

#[derive(Clone, Debug, Deref, DerefMut)]
pub struct P8Map {
    #[deref]
    pub entries: Vec<u8>,
    pub sheet_index: u8,
}

#[derive(Clone, Debug)]
pub enum Map {
    P8(P8Map),
#[cfg(feature = "level")]
    Level(level::Map),
}

impl From<P8Map> for Map {
    fn from(map: P8Map) -> Self {
        Map::P8(map)
    }
}

#[cfg(feature = "level")]
impl From<level::Map> for Map {
    fn from(map: level::Map) -> Self {
        Map::Level(map)
    }
}

#[derive(Clone, Debug)]
pub struct N9Font {
    pub handle: Handle<Font>,
    pub height: Option<f32>,
}

#[derive(Clone, Debug, Deref, DerefMut)]
pub struct AudioBank(pub Vec<Audio>);

#[derive(Debug, Clone)]
pub enum Audio {
    Sfx(Handle<Sfx>),
    AudioSource(Handle<AudioSource>),
}

#[derive(Debug, Clone)]
pub struct Palette {
    pub handle: Handle<Image>,
    pub row: u32,
}

/// Pico8State's state.
#[derive(Resource, Clone)]
pub struct Pico8State {
    pub code: Handle<ScriptAsset>,
    pub(crate) palette: Palette,
    pub(crate) border: Handle<Image>,
    pub(crate) sprite_sheets: Cursor<SpriteSheet>,
    pub(crate) maps: Cursor<Map>,
    // TODO: Let's try to get rid of CART
    // pub(crate) cart: Option<Handle<Cart>>,
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
    Set { sheet: usize },
}

impl FromScript for Spr {
    type This<'w> = Self;
    fn from_script(
        value: ScriptValue,
        _world: WorldAccessGuard<'_>,
    ) -> Result<Self::This<'_>, InteropError> {
        match value {
            ScriptValue::Integer(n) => Ok(if n >= 0 { Spr::Cur { sprite: n as usize } } else { Spr::Set { sheet: n.abs() as usize } }),
            ScriptValue::List(list) => {
                assert_eq!(list.len(), 2, "Expect two elements for spr.");
                let mut iter = list.into_iter().map(|v| match v {
                    ScriptValue::Integer(n) => { Ok(n as usize) }
                    x => Err(InteropError::external_error(Box::new(Error::InvalidArgument(format!("{x:?}").into()))))
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
            Spr::Cur { sprite: index as usize }
        } else {
            Spr::Set { sheet: index.abs().saturating_sub(1) as usize }
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

#[derive(Debug, Clone)]
pub struct SpriteSheet {
    pub handle: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub sprite_size: UVec2,
    pub flags: Vec<u8>,

}

#[derive(Event, Debug)]
struct UpdateCameraPos(UVec2);

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("No asset {0:?} loaded")]
    NoAsset(Cow<'static, str>),
    #[error("texture access error: {0}")]
    TextureAccess(#[from] TextureAccessError),
    #[error("no such button: {0}")]
    NoSuchButton(u8),
    #[error("invalid argument {0}")]
    InvalidArgument(Cow<'static, str>),
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
    carts: ResMut<'w, Assets<Cart>>,
    pub state: ResMut<'w, Pico8State>,
    commands: Commands<'w, 's>,
    canvas: Res<'w, N9Canvas>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    // map: Option<Res<'w, Map>>,
    sfx_channels: Res<'w, SfxChannels>,
    time: Res<'w, Time>,
    // audio_sinks: Query<'w, 's, Option<&'static mut AudioSink>>,
}

#[derive(Debug, Clone, Copy)]
enum SfxCommand {
    Play(u8),
    Release,
    Stop,
}

impl From<u8> for SfxCommand {
    fn from(x: u8) -> Self {
        SfxCommand::Play(x)
    }
}

#[derive(Debug, Clone, Copy)]
enum Radii {
    Radii(u32, u32),
    Radius(u32),
}

impl From<Radii> for UVec2 {
    fn from(r: Radii) -> UVec2 {
        match r {
            Radii::Radii(r1, r2) => UVec2::new(r1, r2),
            Radii::Radius(r) => UVec2::new(r, r),
        }
    }
}

impl Pico8<'_, '_> {
    #[allow(dead_code)]
    fn load_cart(&mut self, cart: Handle<Cart>) {
        self.commands.spawn(LoadCart(cart));
        // self.cart_state.set(CartState::Loading(cart));
    }

    // spr(n, [x,] [y,] [w,] [h,] [flip_x,] [flip_y])
    fn spr(
        &mut self,
        spr: impl Into<Spr>,
        pos: IVec2,
        size: Option<Vec2>,
        flip: Option<BVec2>,
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
        let sprite = {
            let atlas = TextureAtlas {
                layout: sprites.layout.clone(),
                index,
            };
            Sprite {
                image: sprites.handle.clone(),
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
        Ok(self
            .commands
            .spawn((
                Name::new("spr"),
                sprite,
                Transform::from_xyz(x as f32, -y as f32, clearable.suggest_z()),
                clearable,
            ))
            .id())
    }

    pub fn get_color(&self, c: impl Into<N9Color>) -> Result<Color, Error> {
        match c.into() {
            N9Color::Pen => Ok(self.state.draw_state.pen),
            N9Color::Palette(n) => {
                let pal = self
                    .images
                    .get(&self.state.palette.handle)
                    .ok_or(Error::NoAsset("palette".into()))?;

                // Strangely. It's not a 1d texture.
                Ok(pal.get_color_at(n as u32, self.state.palette.row)?)
                //         Ok(c) => Some(c),
                //         Err(e) => {
                //             warn!("Could not look up color in palette at {n}: {e}");
                //             None
                //         }
                //     }
                // })
                // .unwrap_or(Srgba::rgb(1.0, 0.0, 1.0).into())
            }
            N9Color::Color(c) => Ok(c.into()),
        }
    }

    fn cls(&mut self, color: Option<N9Color>) -> Result<(), Error> {
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

    fn pset(&mut self, pos: UVec2, color: Option<N9Color>) -> Result<(), Error> {
        let c = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let image = self
            .images
            .get_mut(&self.canvas.handle)
            .ok_or(Error::NoAsset("canvas".into()))?;
        image.set_color_at(pos.x, pos.y, c)?;
        Ok(())
    }

    fn rectfill(
        &mut self,
        upper_left: UVec2,
        lower_right: UVec2,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let c = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let size = (lower_right - upper_left) + UVec2::ONE;
        let clearable = Clearable::default();
        let id = self
            .commands
            .spawn((
                Name::new("rectfill"),
                Sprite {
                    color: c,
                    anchor: Anchor::TopLeft,
                    custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                    ..default()
                },
                Transform::from_xyz(
                    upper_left.x as f32,
                    -(upper_left.y as f32),
                    clearable.suggest_z(),
                ),
                clearable,
            ))
            .id();
        Ok(id)
    }

    fn rect(
        &mut self,
        upper_left: UVec2,
        lower_right: UVec2,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let c = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let size = (lower_right - upper_left) + UVec2::ONE;
        let clearable = Clearable::default();
        let id = self
            .commands
            .spawn((
                Name::new("rect"),
                Sprite {
                    image: self.state.border.clone(),
                    color: c,
                    anchor: Anchor::TopLeft,
                    custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                    image_mode: SpriteImageMode::Sliced(TextureSlicer {
                        border: BorderRect::square(1.0),
                        center_scale_mode: SliceScaleMode::Stretch,
                        sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1.0 },
                        ..default()
                    }),
                    ..default()
                },
                Transform::from_xyz(
                    upper_left.x as f32,
                    -(upper_left.y as f32),
                    clearable.suggest_z(),
                ),
                clearable,
            ))
            .id();
        Ok(id)
    }

    fn map(
        &mut self,
        map_pos: UVec2,
        screen_start: Vec2,
        size: UVec2,
        mask: Option<u8>,
        map_index: Option<u8>,
    ) -> Result<Entity, Error> {
        let map_size = TilemapSize::from(size);
        // Create a tilemap entity a little early.
        // We want this entity early because we need to tell each tile which tilemap entity
        // it is associated with. This is done with the TilemapId component on each tile.
        // Eventually, we will insert the `TilemapBundle` bundle on the entity, which
        // will contain various necessary components, such as `TileStorage`.

        // To begin creating the map we will need a `TileStorage` component.
        // This component is a grid of tile entities and is used to help keep track of individual
        // tiles in the world. If you have multiple layers of tiles you would have a tilemap entity
        // per layer, each with their own `TileStorage` component.


        // Spawn the elements of the tilemap.
        // Alternatively, you can use helpers::filling::fill_tilemap.
        let clearable = Clearable::default();
        let mut tile_storage = TileStorage::empty(map_size);
        let tilemap_entity = self.commands.spawn(Name::new("map")).id();
        let map_index = 0;//map_index.unwrap_or(0) as usize;
        self.commands
            .entity(tilemap_entity)
            .with_children(|builder| {
                for x in 0..map_size.x {
                    for y in 0..map_size.y {
                        let texture_index = self.state.maps.inner.get(map_index)
                            .and_then(|map| {
                                match map {
                                    Map::P8(ref map) => {
                                        map
                                    .get((map_pos.x + x + (map_pos.y + y) * MAP_COLUMNS) as usize)
                                    .and_then(|index| {
                                        if let Some(mask) = mask {
                                            self.state.sprite_sheets.inner.get(map.sheet_index as usize)
                                                .and_then(|sprite_sheet| (sprite_sheet.flags[*index as usize] & mask == mask).then_some(index))
                                            // (cart.flags[*index as usize] & mask == mask)
                                            //     .then_some(index)
                                        } else {
                                            Some(index)
                                        }
                                    })
                                    }
                                    #[cfg(feature = "level")]
                                    Map::Level(ref map) => {
                                        todo!()
                                    }
                                }
                            })
                            .copied()
                            .unwrap_or(0);
                        if texture_index != 0 {
                            let tile_pos = TilePos {
                                x,
                                y: map_size.y - y - 1,
                            };
                            let tile_entity = builder
                                .spawn((
                                    TileBundle {
                                        position: tile_pos,
                                        tilemap_id: TilemapId(tilemap_entity),
                                        texture_index: TileTextureIndex(texture_index as u32),
                                        ..Default::default()
                                    },
                                    // clearable.clone(),
                                ))
                                .id();
                            tile_storage.set(&tile_pos, tile_entity);
                        }
                    }
                }
            });

        let sprites = &self.state.sprite_sheets;
        let tile_size: TilemapTileSize = sprites.sprite_size.as_vec2().into();
        let grid_size = tile_size.into();
        let map_type = TilemapType::default();
        let mut transform =
            get_tilemap_top_left_transform(&map_size, &grid_size, &map_type, clearable.suggest_z());
        transform.translation += screen_start.extend(0.0);

        self.commands.entity(tilemap_entity).insert((
            TilemapBundle {
                grid_size,
                map_type,
                size: map_size,
                storage: tile_storage,
                texture: TilemapTexture::Single(sprites.handle.clone()),
                tile_size,
                // transform: Transform::from_xyz(screen_start.x, -screen_start.y, 0.0),//get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0),
                transform,
                ..Default::default()
            },
            clearable,
        ));
        Ok(tilemap_entity)
    }

    fn btnp(&self, b: Option<u8>) -> Result<bool, Error> {
        match b {
            Some(b) => Ok(self.keys.just_pressed(match b {
                0 => Ok(KeyCode::ArrowLeft),
                1 => Ok(KeyCode::ArrowRight),
                2 => Ok(KeyCode::ArrowUp),
                3 => Ok(KeyCode::ArrowDown),
                4 => Ok(KeyCode::KeyZ),
                5 => Ok(KeyCode::KeyX),
                x => Err(Error::NoSuchButton(x)),
            }?)),
            // None => Ok(!self.keys.get_just_pressed().is_empty())
            None => Ok(self.keys.get_just_pressed().len() != 0),
        }
    }

    #[allow(dead_code)]
    fn btn(&self, b: Option<u8>) -> Result<bool, Error> {
        match b {
            Some(b) => Ok(self.keys.pressed(match b {
                0 => Ok(KeyCode::ArrowLeft),
                1 => Ok(KeyCode::ArrowRight),
                2 => Ok(KeyCode::ArrowUp),
                3 => Ok(KeyCode::ArrowDown),
                4 => Ok(KeyCode::KeyZ),
                5 => Ok(KeyCode::KeyX),
                x => Err(Error::NoSuchButton(x)),
            }?)),
            // None => Ok(!self.keys.get_pressed().is_empty())
            None => Ok(self.keys.get_pressed().len() != 0),
        }
    }

    // print(text, [x,] [y,] [color])
    fn print(
        &mut self,
        text: impl AsRef<str>,
        pos: Option<UVec2>,
        color: Option<N9Color>,
    ) -> Result<u32, Error> {
        const CHAR_WIDTH: u32 = 4;
        const NEWLINE_HEIGHT: u32 = 6;
        let mut text: &str = text.as_ref();
        // warn!("PRINTING {}", &text);
        // info!("print {:?} start, {:?}", &text, &self.state.draw_state.print_cursor);
        let pos = pos.unwrap_or_else(|| {
            UVec2::new(
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
        let len = text.len() as u32;
        let z = clearable.suggest_z();
        self.commands
            .spawn((
                Name::new("print"),
                Transform::from_xyz(pos.x as f32, -(pos.y as f32), z),
                Visibility::default(),
                clearable,
            ))
            .with_children(|builder| {
                let mut y = 0;
                for line in text.lines() {
                    // Our font has a different height than we want. It's one pixel
                    // higher. So we can't let bevy render it one go. Bummer.
                    builder.spawn((
                        Text2d::new(line),
                        Transform::from_xyz(0.0, -(y as f32), z),
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
    fn sfx(
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
                // let cart = self
                //     .state
                //     .cart
                //     .as_ref()
                //     .and_then(|cart| self.carts.get(cart))
                //     .expect("cart");
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

    fn fget(&self, index: u8, flag_index: Option<u8>) -> u8 {
        let flags = &self.state.sprite_sheets.flags;
        // let cart = self
        //     .state
        //     .cart
        //     .as_ref()
        //     .and_then(|cart| self.carts.get(cart))
        //     .expect("cart");
        if let Some(v) = flags.get(index as usize) {
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
                warn_once!("No flags present for cart.");
            } else {
                warn!(
                    "Requested flag at {index}. There are only {} flags.",
                    flags.len()
                );
            }
            0
        }
    }

    fn fset(&mut self, index: u8, flag_index: Option<u8>, value: u8) {
        let mut flags = &mut self.state.sprite_sheets.flags;
        // let cart = self
        //     .state
        //     .cart
        //     .as_ref()
        //     .and_then(|cart| self.carts.get_mut(cart))
        //     .expect("cart");
        match flag_index {
            Some(flag_index) => {
                let v = flags[index as usize];
                if value != 0 {
                    // Set the bit.
                    flags[index as usize] |= 1 << flag_index;
                } else {
                    // Unset the bit.
                    flags[index as usize] &= !(1 << flag_index);
                }
            }
            None => {
                flags[index as usize] = value;
            }
        }
    }

    fn mget(&self, pos: UVec2) -> u8 {
        let map: &Map = &self.state.maps;
        // let cart = self
        //     .state
        //     .cart
        //     .as_ref()
        //     .and_then(|cart| self.carts.get(cart))
        //     .expect("cart");
        match *map {
            Map::P8(ref map) => map[(pos.x + pos.y * MAP_COLUMNS) as usize],

            #[cfg(feature = "level")]
            Map::Level(ref map) => todo!()
        }
    }

    fn mset(&mut self, pos: UVec2, sprite_index: u8) {
        let mut map: &mut Map = &mut self.state.maps;
        // let cart = self
        //     .state
        //     .cart
        //     .as_ref()
        //     .and_then(|cart| self.carts.get_mut(cart))
        //     .expect("cart");
        match *map {
            Map::P8(ref mut map) => {
                map[(pos.x + pos.y * MAP_COLUMNS) as usize] = sprite_index;
            }
            #[cfg(feature = "level")]
            Map::Level(ref mut map) => {
                todo!()
            }
        }
    }

    fn sub(string: &str, start: isize, end: Option<isize>) -> String {
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

    fn time(&self) -> f32 {
        self.time.elapsed_secs()
    }

    fn camera(&mut self, pos: UVec2) -> UVec2 {
        let result = std::mem::replace(&mut self.state.draw_state.camera_position, pos);
        self.commands.trigger(UpdateCameraPos(pos));
        result
    }

    // fn line(&mut self, pos0: UVec2, pos1: UVec2, color: Option<N9Color>) {

    // }

    fn line(&mut self, a: IVec2, b: IVec2, color: Option<N9Color>) -> Result<Entity, Error> {
        let color = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let min = a.min(b);
        let delta = b - a;
        // let size = UVec2::new((a.x - b.x).abs() + 1,
        //                       (a.y - b.y).abs() + 1);
        let size = UVec2::new(delta.x.abs() as u32, delta.y.abs() as u32) + UVec2::ONE;
        // dbg!(a, b, size);
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
            // dbg!(x, y);
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
                    // image_mode: SpriteImageMode::Sliced(TextureSlicer {
                    //     border: BorderRect::square(1.0),
                    //     center_scale_mode: SliceScaleMode::Stretch,
                    //     sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1.0 },
                    //     ..default()
                    // }),
                    ..default()
                },
                Transform::from_xyz(min.x as f32, -(min.y as f32), clearable.suggest_z()),
                clearable,
            ))
            .id();
        Ok(id)
    }

    fn rnd(&mut self, value: ScriptValue) -> ScriptValue {
        let mut rng = rand::thread_rng();
        match value {
            ScriptValue::Integer(x) => ScriptValue::from(rng.gen_range(0..=x)),
            ScriptValue::Float(x) => ScriptValue::from(rng.gen_range(0.0..x)),
            ScriptValue::List(mut x) => {
                if x.is_empty() {
                    ScriptValue::Unit
                } else {
                    let index = rng.gen_range(0..x.len());
                    x.swap_remove(index)
                }
            }
            _ => ScriptValue::Error(InteropError::external_error(Box::new(
                Error::InvalidArgument("rng expects integer, float, or list".into()),
            ))),
        }
    }

    fn circfill(
        &mut self,
        pos: IVec2,
        r: impl Into<UVec2>,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let color = self.get_color(color.unwrap_or(N9Color::Pen))?;
        // let min = a.min(b);
        let r: UVec2 = r.into();
        let size: UVec2 = r * UVec2::splat(2) + UVec2::ONE;
        // // let size = UVec2::new((a.x - b.x).abs() + 1,
        // //                       (a.y - b.y).abs() + 1);
        // let size = UVec2::new(delta.x.abs() as u32, delta.y.abs() as u32) + UVec2::ONE;
        // dbg!(a, b, size);
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
                    // image_mode: SpriteImageMode::Sliced(TextureSlicer {
                    //     border: BorderRect::square(1.0),
                    //     center_scale_mode: SliceScaleMode::Stretch,
                    //     sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1.0 },
                    //     ..default()
                    // }),
                    ..default()
                },
                Transform::from_xyz(pos.x as f32, -(pos.y as f32), clearable.suggest_z()),
                clearable,
            ))
            .id();
        Ok(id)
    }

    fn circ(
        &mut self,
        pos: IVec2,
        r: impl Into<UVec2>,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let color = self.get_color(color.unwrap_or(N9Color::Pen))?;
        // let min = a.min(b);
        let r: UVec2 = r.into();
        let size: UVec2 = r * UVec2::splat(2) + UVec2::ONE;
        // // let size = UVec2::new((a.x - b.x).abs() + 1,
        // //                       (a.y - b.y).abs() + 1);
        // let size = UVec2::new(delta.x.abs() as u32, delta.y.abs() as u32) + UVec2::ONE;
        // dbg!(a, b, size);
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
                    // image_mode: SpriteImageMode::Sliced(TextureSlicer {
                    //     border: BorderRect::square(1.0),
                    //     center_scale_mode: SliceScaleMode::Stretch,
                    //     sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1.0 },
                    //     ..default()
                    // }),
                    ..default()
                },
                Transform::from_xyz(pos.x as f32, -(pos.y as f32), clearable.suggest_z()),
                clearable,
            ))
            .id();
        Ok(id)
    }

    fn ovalfill(
        &mut self,
        upper_left: IVec2,
        lower_right: IVec2,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let color = self.get_color(color.unwrap_or(N9Color::Pen))?;
        // let min = a.min(b);
        let size: UVec2 = ((lower_right - upper_left) + IVec2::ONE).try_into().unwrap();
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
        let handle = self.images.add(image);
        let clearable = Clearable::default();
        let offset = 0.5;
        let id = self
            .commands
            .spawn((
                Name::new("ovalfill"),
                Sprite {
                    image: handle,
                    color,
                    anchor: Anchor::TopLeft,
                    custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                    // image_mode: SpriteImageMode::Sliced(TextureSlicer {
                    //     border: BorderRect::square(1.0),
                    //     center_scale_mode: SliceScaleMode::Stretch,
                    //     sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1.0 },
                    //     ..default()
                    // }),
                    ..default()
                },
                Transform::from_xyz(upper_left.x as f32, -(upper_left.y as f32), clearable.suggest_z()),
                clearable,
            ))
            .id();
        Ok(id)
    }

    fn oval(
        &mut self,
        upper_left: IVec2,
        lower_right: IVec2,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let color = self.get_color(color.unwrap_or(N9Color::Pen))?;
        // let min = a.min(b);
        let size: UVec2 = ((lower_right - upper_left) + IVec2::ONE).try_into().unwrap();
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

        let offset = 0.5;
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
                    // image_mode: SpriteImageMode::Sliced(TextureSlicer {
                    //     border: BorderRect::square(1.0),
                    //     center_scale_mode: SliceScaleMode::Stretch,
                    //     sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1.0 },
                    //     ..default()
                    // }),
                    ..default()
                },
                Transform::from_xyz(upper_left.x as f32, -(upper_left.y as f32), clearable.suggest_z()),
                clearable,
            ))
            .id();
        Ok(id)
    }
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
                                Audio::AudioSource(source) => {
                                    todo!();
                                }
                            }
                        } else {
                            warn!("Channels busy.");
                            // Err(Error::ChannelsBusy)?;
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
                            Audio::AudioSource(source) => {
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

/// Calculates a [`Transform`] for a tilemap that places it so that its center is at
/// `(0.0, 0.0, 0.0)` in world space.
fn get_tilemap_top_left_transform(
    size: &TilemapSize,
    grid_size: &TilemapGridSize,
    map_type: &TilemapType,
    z: f32,
) -> Transform {
    assert_eq!(map_type, &TilemapType::Square);
    let y = size.y as f32 * grid_size.y;
    Transform::from_xyz(grid_size.x / 2.0, -y + grid_size.y / 2.0, z)
}

impl FromWorld for Pico8State {
    fn from_world(world: &mut World) -> Self {
        let layout = {
            let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
            layouts.add(TextureAtlasLayout::from_grid(
                PICO8_SPRITE_SIZE,
                PICO8_TILE_COUNT.x,
                PICO8_TILE_COUNT.y,
                None,
                None,
            ))
        };
        let asset_server = world.resource::<AssetServer>();

        let pixel_art_settings = |settings: &mut ImageLoaderSettings| {
            // Use `nearest` image sampling to preserve the pixel art style.
            settings.sampler = ImageSampler::nearest();
        };

        Pico8State {
            palette: Palette { handle: asset_server.load_with_settings(PICO8_PALETTE, pixel_art_settings),
                               row: 0 },
            border: asset_server.load_with_settings(PICO8_BORDER, pixel_art_settings),
            code: Handle::<ScriptAsset>::default(),
            font: vec![N9Font {
                handle: asset_server.load(PICO8_FONT),
                height: Some(7.0),
            }].into(),
            draw_state: DrawState::default(),
            audio_banks: Vec::new().into(),
            sprite_sheets: Vec::new().into(),
            maps: Vec::new().into(),
        }
    }
}

impl Pico8State {
    pub fn get_color_or_pen(&self, c: impl Into<N9Color>, world: &World) -> Color {
        match c.into() {
            N9Color::Pen => self.draw_state.pen,
            N9Color::Palette(n) => {
                let images = world.resource::<Assets<Image>>();
                images
                    .get(&self.palette.handle)
                    .and_then(|pal| {
                        // Strangely. It's not a 1d texture.
                        match pal.get_color_at(n as u32, self.palette.row) {
                            Ok(c) => Some(c),
                            Err(e) => {
                                warn!("Could not look up color in palette at {n}: {e}");
                                None
                            }
                        }
                    })
                    .unwrap_or(Srgba::rgb(1.0, 0.0, 1.0).into())
            }
            N9Color::Color(c) => c.into(),
        }
    }

    // pub fn get_color(index: usize, world: &mut World) -> Result<Color, LuaError> {
    //     let mut system_state: SystemState<(Res<Nano9Palette>, Res<Assets<Image>>, Res<DrawState>)> =
    //         SystemState::new(world);
    //     let (palette, images, draw_state) = system_state.get(world);

    //     images
    //         .get(&palette.0)
    //         .ok_or_else(|| LuaError::RuntimeError(format!("no such palette {:?}", &palette.0)))
    //         .and_then(|pal| {
    //             pal.get_color_at_1d(index as u32)
    //                 .map_err(|_| LuaError::RuntimeError(format!("no such pixel index {:?}", index)))
    //         })
    // }
}

pub struct Pico8API;

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<Pico8State>()
        .add_plugins(attach_api)
        .add_systems(
            PreStartup,
            |mut asset_settings: ResMut<ScriptAssetSettings>| {
                fn path_to_lang(path: &Path) -> Language {
                    // For carts we use cart.p8#lua, which is labeled asset, so we
                    // need to tell it what language our cart is.
                    if path.to_str().map(|s| s.ends_with("lua")).unwrap_or(false) {
                        Language::Lua
                    } else {
                        Language::Unknown
                    }
                }
                asset_settings
                    .script_language_mappers
                    .push(AssetPathToLanguageMapper { map: path_to_lang });
            },
        )
        .add_observer(
            |trigger: Trigger<UpdateCameraPos>,
             camera: Single<&mut Transform, With<Nano9Camera>>| {
                let pos = trigger.event();
                let mut camera = camera.into_inner();
                camera.translation.x = pos.0.x as f32;
                camera.translation.y = -(pos.0.y as f32);
            },
        );
}

fn with_pico8<X>(
    ctx: &FunctionCallContext,
    f: impl FnOnce(&mut Pico8) -> Result<X, Error>,
) -> Result<X, InteropError> {
    let world_guard = ctx.world()?;
    let raid = ReflectAccessId::for_global();
    if world_guard.claim_global_access() {
        let world = world_guard.as_unsafe_world_cell()?;
        let world = unsafe { world.world_mut() };
        let mut system_state: SystemState<Pico8> = SystemState::new(world);
        let mut pico8 = system_state.get_mut(world);
        let r = f(&mut pico8);
        system_state.apply(world);
        unsafe { world_guard.release_global_access() };
        r.map_err(|e| InteropError::external_error(Box::new(e)))
    } else {
        Err(InteropError::cannot_claim_access(
            raid,
            world_guard.get_access_location(raid),
            "with_pico8",
        ))
    }
}

fn attach_api(app: &mut App) {
    // callbacks can receive any `ToLuaMulti` arguments, here '()' and
    // return any `FromLuaMulti` arguments, here a `usize`
    // check the Rlua documentation for more details
    let world = app.world_mut();

    NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
        .register("btnp", |ctx: FunctionCallContext, b: Option<u8>| {
            with_pico8(&ctx, |pico8| pico8.btnp(b))
        })
        .register("btn", |ctx: FunctionCallContext, b: Option<u8>| {
            with_pico8(&ctx, |pico8| pico8.btn(b))
        })
        .register("cls", |ctx: FunctionCallContext, c: Option<N9Color>| {
            with_pico8(&ctx, |pico8| pico8.cls(c))
        })
        .register(
            "pset",
            |ctx: FunctionCallContext, x: u32, y: u32, color: Option<N9Color>| {
                with_pico8(&ctx, |pico8| {
                    // We want to ignore out of bounds errors specifically but possibly not others.
                    // Ok(pico8.pset(x, y, color)?)
                    let _ = pico8.pset(UVec2::new(x, y), color);
                    Ok(())
                })
            },
        )
        .register(
            "rectfill",
            |ctx: FunctionCallContext,
             x0: u32,
             y0: u32,
             x1: u32,
             y1: u32,
             color: Option<N9Color>| {
                with_pico8(&ctx, |pico8| {
                    // We want to ignore out of bounds errors specifically but possibly not others.
                    // Ok(pico8.pset(x, y, color)?)
                    let _ = pico8.rectfill(UVec2::new(x0, y0), UVec2::new(x1, y1), color);
                    Ok(())
                })
            },
        )
        .register(
            "rect",
            |ctx: FunctionCallContext,
             x0: u32,
             y0: u32,
             x1: u32,
             y1: u32,
             color: Option<N9Color>| {
                with_pico8(&ctx, |pico8| {
                    // We want to ignore out of bounds errors specifically but possibly not others.
                    // Ok(pico8.pset(x, y, color)?)
                    let _ = pico8.rect(UVec2::new(x0, y0), UVec2::new(x1, y1), color);
                    Ok(())
                })
            },
        )
        // spr(n, [x,] [y,] [w,] [h,] [flip_x,] [flip_y])
        .register(
            "spr",
            |ctx: FunctionCallContext,
             n: ScriptValue,
             x: Option<f32>,
             y: Option<f32>,
             w: Option<f32>,
             h: Option<f32>,
             flip_x: Option<bool>,
             flip_y: Option<bool>| {

                let pos = IVec2::new(
                    x.map(|a| a.round() as i32).unwrap_or(0),
                    y.map(|a| a.round() as i32).unwrap_or(0),
                );
                let flip = (flip_x.is_some() || flip_y.is_some())
                    .then(|| BVec2::new(flip_x.unwrap_or(false), flip_y.unwrap_or(false)));
                let size = w
                    .or(h)
                    .is_some()
                    .then(|| Vec2::new(w.unwrap_or(1.0), h.unwrap_or(1.0)));

                // We get back an entity. Not doing anything with it here yet.
                let n = Spr::from_script(n, ctx.world()?)?;
                let _id = with_pico8(&ctx, move |pico8| pico8.spr(n, pos, size, flip))?;
                Ok(())
            },
        )
        // map( celx, cely, sx, sy, celw, celh, [layer] )
        .register(
            "map",
            |ctx: FunctionCallContext,
             celx: Option<u32>,
             cely: Option<u32>,
             sx: Option<f32>,
             sy: Option<f32>,
             celw: Option<u32>,
             celh: Option<u32>,
             layer: Option<u8>,
             map_index: Option<u8>| {
                let id = with_pico8(&ctx, move |pico8| {
                    pico8.map(
                        UVec2::new(celx.unwrap_or(0), cely.unwrap_or(0)),
                        Vec2::new(sx.unwrap_or(0.0), sy.unwrap_or(0.0)),
                        UVec2::new(celw.unwrap_or(16), celh.unwrap_or(16)),
                        layer,
                        map_index,
                    )
                })?;

                let entity = N9Entity {
                    entity: id,
                    drop: DropPolicy::Nothing,
                };
                let world = ctx.world()?;
                let reference = {
                    let allocator = world.allocator();
                    let mut allocator = allocator.write();
                    ReflectReference::new_allocated(entity, &mut allocator)
                };
                Ok(ReflectReference::into_script_ref(reference, world)?)
            },
        )
        .register(
            "print",
            |ctx: FunctionCallContext,
             text: Option<String>,
             x: Option<u32>,
             y: Option<u32>,
             c: Option<N9Color>| {
                with_pico8(&ctx, move |pico8| {
                    let pos = x
                        .map(|x| UVec2::new(x, y.unwrap_or(pico8.state.draw_state.print_cursor.y)));
                    pico8.print(text.as_deref().unwrap_or(""), pos, c)
                })
            },
        )
        .register(
            "sfx",
            |ctx: FunctionCallContext,
             n: i8,
             channel: Option<u8>,
             offset: Option<u8>,
             length: Option<u8>| {
                with_pico8(&ctx, move |pico8| {
                    pico8.sfx(
                        match n {
                            -2 => Ok(SfxCommand::Release),
                            -1 => Ok(SfxCommand::Stop),
                            n if n >= 0 => Ok(SfxCommand::Play(n as u8)),
                            x => {
                                // Maybe we should let Lua errors pass through.
                                // Err(LuaError::BadArgument {
                                //     to: Some("sfx".into()),
                                //     pos: 0,
                                //     name: Some("n".into()),
                                //     cause: std::sync::Arc::new(
                                // })
                                Err(Error::InvalidArgument(
                                    format!("sfx: expected n to be -2, -1 or >= 0 but was {x}")
                                        .into(),
                                ))
                            }
                        }?,
                        channel,
                        offset,
                        length,
                        None,
                    )
                })
            },
        )
        .register("fget", |ctx: FunctionCallContext, n: u8, f: Option<u8>| {
            with_pico8(&ctx, move |pico8| Ok(pico8.fget(n, f)))
        })
        .register(
            "fset",
            |ctx: FunctionCallContext, n: u8, f_or_v: u8, v: Option<u8>| {
                let (f, v) = v.map(|v| (Some(f_or_v), v)).unwrap_or((None, f_or_v));
                with_pico8(&ctx, move |pico8| {
                    pico8.fset(n, f, v);
                    Ok(())
                })
            },
        )
        .register("mget", |ctx: FunctionCallContext, x: u32, y: u32| {
            with_pico8(&ctx, move |pico8| Ok(pico8.mget(UVec2::new(x, y))))
        })
        .register("mset", |ctx: FunctionCallContext, x: u32, y: u32, v: u8| {
            with_pico8(&ctx, move |pico8| {
                pico8.mset(UVec2::new(x, y), v);
                Ok(())
            })
        })
        .register("sub", |s: String, start: isize, end: Option<isize>| {
            Pico8::sub(&s, start, end)
        })
        .register("time", |ctx: FunctionCallContext| {
            with_pico8(&ctx, move |pico8| Ok(pico8.time()))
        })
        .register("rnd", |ctx: FunctionCallContext, value: ScriptValue| {
            with_pico8(&ctx, move |pico8| Ok(pico8.rnd(value)))
        })
        .register(
            "camera",
            |ctx: FunctionCallContext, x: Option<u32>, y: Option<u32>| {
                with_pico8(&ctx, move |pico8| {
                    Ok(pico8.camera(UVec2::new(x.unwrap_or(0), y.unwrap_or(0))))
                })
                .map(|last_pos| (last_pos.x, last_pos.y))
            },
        )
        .register(
            "line",
            |ctx: FunctionCallContext,
             x0: Option<i32>,
             y0: Option<i32>,
             x1: Option<i32>,
             y1: Option<i32>,
             c: Option<N9Color>| {
                let _ = with_pico8(&ctx, move |pico8| {
                    pico8.line(
                        IVec2::new(x0.unwrap_or(0), y0.unwrap_or(0)),
                        IVec2::new(x1.unwrap_or(0), y1.unwrap_or(0)),
                        c,
                    )
                })?;
                Ok(())
            },
        )
        .register(
            "circfill",
            |ctx: FunctionCallContext,
             x0: Option<i32>,
             y0: Option<i32>,
             r: Option<u32>,
             c: Option<N9Color>| {
                let _ = with_pico8(&ctx, move |pico8| {
                    pico8.circfill(
                        IVec2::new(x0.unwrap_or(0), y0.unwrap_or(0)),
                        UVec2::splat(r.unwrap_or(4)),
                        c,
                    )
                })?;
                Ok(())
            },
        )
        .register(
            "circ",
            |ctx: FunctionCallContext,
             x0: Option<i32>,
             y0: Option<i32>,
             r: Option<u32>,
             c: Option<N9Color>| {
                let _ = with_pico8(&ctx, move |pico8| {
                    pico8.circ(
                        IVec2::new(x0.unwrap_or(0), y0.unwrap_or(0)),
                        UVec2::splat(r.unwrap_or(4)),
                        c,
                    )
                })?;
                Ok(())
            },
        )
        .register(
            "ovalfill",
            |ctx: FunctionCallContext,
             x0: Option<i32>,
             y0: Option<i32>,
             x1: Option<i32>,
             y1: Option<i32>,
             c: Option<N9Color>| {
                let _ = with_pico8(&ctx, move |pico8| {
                    pico8.ovalfill(
                        IVec2::new(x0.unwrap_or(0), y0.unwrap_or(0)),
                        IVec2::new(x1.unwrap_or(0), y1.unwrap_or(0)),
                        c,
                    )
                })?;
                Ok(())
            },
        )
        .register(
            "oval",
            |ctx: FunctionCallContext,
             x0: Option<i32>,
             y0: Option<i32>,
             x1: Option<i32>,
             y1: Option<i32>,
             c: Option<N9Color>| {
                let _ = with_pico8(&ctx, move |pico8| {
                    pico8.oval(
                        IVec2::new(x0.unwrap_or(0), y0.unwrap_or(0)),
                        IVec2::new(x1.unwrap_or(0), y1.unwrap_or(0)),
                        c,
                    )
                })?;
                Ok(())
            },
        );

    //     fn tostr(ctx, v: Value) {
    //         let tostring: Function = ctx.globals().get("tostring")?;
    //         tostring.call::<Value,LuaString>(v)
    //     }

    //     fn flr(ctx, v: Number) {
    //         Ok(v.floor() as u32)
    //     }

    //     fn sub(ctx, (string, start, end): (LuaString, isize, Option<isize>)) {
    //         let string = string.to_str()?;
    //         let start = if start < 0 {
    //             (string.len() as isize - start - 1) as usize
    //         } else {
    //             (start - 1) as usize
    //         };
    //         match end {
    //             Some(end) => {
    //                 let end = if end < 0 {
    //                     (string.len() as isize - end) as usize
    //                 } else {
    //                     end as usize
    //                 };
    //                 if start <= end {
    //                     Ok(string.chars().skip(start).take(end - start).collect())
    //                     // BUG: This cuts unicode boundaries.
    //                     // Ok(string[start..end].to_string())
    //                 } else {
    //                     Ok(String::new())
    //                 }
    //             }
    //             None => Ok(string.chars().skip(start).collect())
    //         }
    //     }

    //     fn min(ctx, (x, y): (Value, Value)) {
    //         Ok(if x.to_f32() < y.to_f32() {
    //             x
    //         } else {
    //             y
    //         })
    //     }

    //     fn max(ctx, (x, y): (Value, Value)) {
    //         Ok(if x.to_f32() > y.to_f32() {
    //             x
    //         } else {
    //             y
    //         })
    //     }

    //     fn ord(ctx, (string, index, count): (LuaString, Option<usize>, Option<usize>)) {
    //         let string = string.to_str()?;
    //         let index = index.map(|i| i - 1).unwrap_or(0);
    //         let count = count.unwrap_or(1);
    //         let mut result: Vec<Value> = Vec::with_capacity(count);
    //         for c in string.chars().skip(index).take(count) {
    //             result.push(Value::Integer(c as i64));
    //         }
    //         Ok(LuaMultiValue::from_vec(result))
    //     }
    // }

    // Ok(())
}

//     fn register_with_app(&self, _app: &mut App) {
//         // app.register_type::<Settings>();
//     }
// }

#[cfg(test)]
mod test {

    #[test]
    fn test_suffix_match() {
        let s = "a\\0";
        assert_eq!(s.len(), 3);
        assert!(s.ends_with("\\0"));
    }
}

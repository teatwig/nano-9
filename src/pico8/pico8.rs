use bevy::{
    ecs::{system::{SystemState, SystemParam}, world::Command},
    image::{ImageLoaderSettings, ImageSampler, TextureAccessError},
    prelude::*,
    sprite::Anchor,
};

use bevy_mod_scripting::lua::prelude::tealr::mlu::mlua::{
    Function, Number
};

use bevy_mod_scripting::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    pico8::{LoadCart, Cart, Clearable, ClearEvent},
    api::N9Args, N9Color, Nano9Screen, ValueExt, DrawState,
};

use std::{
    sync::Mutex,
    borrow::Cow,
};

pub const PICO8_PALETTE: &str = "images/pico-8-palette.png";
pub const PICO8_SPRITES: &str = "images/pooh-book-sprites.png";
pub const PICO8_BORDER: &str = "images/rect-border.png";
pub const PICO8_FONT: &str = "fonts/pico-8.ttf";

/// Pico8State's state.
#[derive(Resource, Clone)]
pub struct Pico8State {
    pub(crate) palette: Handle<Image>,
    pub(crate) border: Handle<Image>,
    pub(crate) sprites: Handle<Image>,
    pub(crate) cart: Option<Handle<Cart>>,
    pub(crate) layout: Handle<TextureAtlasLayout>,
    pub(crate) font: Handle<Font>,
    pub(crate) draw_state: DrawState,
    pub(crate) sprite_size: UVec2,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("No asset {0:?} loaded")]
    NoAsset(Cow<'static, str>),
    #[error("texture access error: {0}")]
    TextureAccess(#[from] TextureAccessError),
    #[error("no such button: {0}")]
    NoSuchButton(u8),
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
    state: ResMut<'w, Pico8State>,
    commands: Commands<'w, 's>,
    background: Res<'w, Nano9Screen>,
    keys: Res<'w, ButtonInput<KeyCode>>,
}

#[derive(Default, Clone, Copy)]
pub struct SprArgs {
    pos: Vec2,
    size: Option<Vec2>,
    flip_x: bool,
    flip_y: bool,
}



impl Pico8<'_, '_> {
    fn load_cart(&mut self, cart: Handle<Cart>) {
        self.commands.spawn(LoadCart(cart));
        // self.cart_state.set(CartState::Loading(cart));
    }

    // spr(n, [x,] [y,] [w,] [h,] [flip_x,] [flip_y])

    // XXX: Reconsider using args struct.
    // fn spr(&mut self, index: usize, pos: Option<Vec2>, size: Option<Vec2>, flip: Option<BVec2>) -> Result<Entity, Error> {
    fn spr(&mut self, index: usize, args: Option<SprArgs>) -> Result<Entity, Error> {
        let args = args.unwrap_or_default();
        let x = args.pos.x;
        let y = args.pos.y;
        let flip_x = args.flip_x;
        let flip_y = args.flip_y;
        let sprite = {
            let atlas = TextureAtlas {
                layout: self.state.layout.clone(),
                index,
            };
            Sprite {
                image: self.state.sprites.clone(),
                texture_atlas: Some(atlas),
                rect: args.size.map(|v|
                                    Rect { min: Vec2::ZERO,
                                           max: self.state.sprite_size.as_vec2() * v }),
                flip_x,
                flip_y,
                ..default()
            }
        };
        let clearable = Clearable::default();
        Ok(self.commands.spawn((
            Name::new("sprite"),
            sprite,
                        Transform::from_xyz(x, -y, clearable.suggest_z()),
            clearable,
        )).id())
    }

    pub fn get_color(&self, c: impl Into<N9Color>) -> Result<Color, Error> {
        match c.into() {
            N9Color::Pen => Ok(self.state.draw_state.pen),
            N9Color::Palette(n) => {
                let pal = self.images.get(&self.state.palette).ok_or(Error::NoAsset("palette".into()))?;

                    // Strangely. It's not a 1d texture.
                Ok(pal.get_color_at(n as u32, 0)?)
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
        let image = self.images.get_mut(&self.background.0).ok_or(Error::NoAsset("background".into()))?;
        for i in 0..image.width() {
            for j in 0..image.height() {
                image.set_color_at(i, j, c)?;
            }
        }
        self.commands.send_event(ClearEvent::default());
        Ok(())
    }

    fn pset(&mut self, x: u32, y: u32, color: Option<N9Color>) -> Result<(), Error> {
        let c = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let image = self.images.get_mut(&self.background.0).ok_or(Error::NoAsset("background".into()))?;
        image.set_color_at(x, y, c)?;
        Ok(())
    }

    fn rectfill(&mut self, upperLeft: UVec2, lowerRight: UVec2, color: Option<N9Color>) -> Result<Entity, Error> {
        let c = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let size = lowerRight - upperLeft;
        let clearable = Clearable::default();
        let id = self.commands.spawn((
            Name::new("rectfill"),
            Sprite {
                color: c,
                anchor: Anchor::TopLeft,
                custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                ..default()
            },
            Transform::from_xyz(upperLeft.x as f32, -(upperLeft.y as f32), clearable.suggest_z()),
            clearable,
            ))
                     .id();
        Ok(id)
    }

    fn rect(&mut self, upperLeft: UVec2, lowerRight: UVec2, color: Option<N9Color>) -> Result<Entity, Error> {
        let c = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let size = lowerRight - upperLeft;
        let clearable = Clearable::default();
        let id = self.commands.spawn((
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
            Transform::from_xyz(upperLeft.x as f32, -(upperLeft.y as f32), clearable.suggest_z()),
            clearable,
            ))
                     .id();
        Ok(id)
    }


    fn map(&mut self, map_pos: UVec2, screen_start: Vec2, size: UVec2, mask: u8) -> Result<Entity, Error> {
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

        let cart = self.state.cart.as_ref().and_then(|cart| self.carts.get(cart));

        // Spawn the elements of the tilemap.
        // Alternatively, you can use helpers::filling::fill_tilemap.
        let clearable = Clearable::default();
        let mut tile_storage = TileStorage::empty(map_size);
        let tilemap_entity = self.commands.spawn(Name::new("map")).id();
        self.commands.entity(tilemap_entity)
            .with_children(|builder| {
        for x in 0..map_size.x {
            for y in 0..map_size.y {
                let texture_index = cart.and_then(|cart| cart.map.get((map_pos.x + x + (map_pos.y + y) * 128) as usize)).copied().unwrap_or(0);
                if texture_index != 0 {
                    let tile_pos = TilePos { x, y: map_size.y - y - 1};
                    let tile_entity = builder
                                          .spawn((TileBundle {
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

        let tile_size = TilemapTileSize { x: 8.0, y: 8.0 };
        let grid_size = tile_size.into();
        let map_type = TilemapType::default();

        self.commands.entity(tilemap_entity).insert((TilemapBundle {
            grid_size,
            map_type,
            size: map_size,
            storage: tile_storage,
            texture: TilemapTexture::Single(self.state.sprites.clone()),
            tile_size,
            // transform: Transform::from_xyz(screen_start.x, -screen_start.y, 0.0),//get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0),
            transform: get_tilemap_top_left_transform(&map_size, &grid_size, &map_type, clearable.suggest_z()),
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
            None => Ok(self.keys.get_just_pressed().len() != 0)
        }
    }

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
            None => Ok(self.keys.get_pressed().len() != 0)
        }
    }

    // print(text, [x,] [y,] [color])
    fn print(&mut self, text: impl AsRef<str>, pos: Option<UVec2>, color: Option<N9Color>) -> Result<u32, Error> {
        const CHAR_WIDTH: u32 = 4;
        const NEWLINE_HEIGHT: u32 = 6;
        let mut text: &str = text.as_ref();
        // info!("print {:?} start, {:?}", &text, &self.state.draw_state.print_cursor);
        let pos = pos.unwrap_or_else(|| UVec2::new(self.state.draw_state.print_cursor.x, self.state.draw_state.print_cursor.y));
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
        self.commands.spawn((Name::new("print"),
                             Transform::from_xyz(pos.x as f32, -(pos.y as f32), z),
                             Visibility::default(),
                             clearable))
            .with_children(|builder| {
                let mut y = 0;
                for line in text.lines() {
                // Our font has a different height than we want. It's one pixel
                // higher. So we can't let bevy render it one go. Bummer.
                builder.spawn((Text2d::new(line),
                               Transform::from_xyz(0.0, -(y as f32), z),

                                    TextColor(c),
                                    TextFont {
                                        font: self.state.font.clone(),
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
    fn sfx(&mut self, n: u8, channel: Option<u8>, offset: Option<u8>, length: Option<u8>) -> Result<(), Error> {
        let cart = self.state.cart.as_ref().and_then(|cart| self.carts.get(cart)).expect("cart");
        let sfx = cart.sfx.get(n as usize).ok_or(Error::NoAsset(format!("sfx {n}").into()))?;

        self.commands.spawn((Name::new("sfx"),
                             AudioPlayer(sfx.clone()),
                             PlaybackSettings::DESPAWN));
        Ok(())
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
            layouts.add(TextureAtlasLayout::from_grid(UVec2::new(8, 8),
                                                      16,
                                                      16,
                                                      None,
                                                      None))
        };
        let asset_server = world.resource::<AssetServer>();

        let pixel_art_settings = |settings: &mut ImageLoaderSettings| {
            // Use `nearest` image sampling to preserve the pixel art style.
            settings.sampler = ImageSampler::nearest();
        };

        Pico8State {
            palette: asset_server.load_with_settings(PICO8_PALETTE, pixel_art_settings),
            sprites: asset_server.load_with_settings(PICO8_SPRITES, pixel_art_settings),
            border: asset_server.load_with_settings(PICO8_BORDER, pixel_art_settings),
            sprite_size: UVec2::splat(8),
            cart: None,
            layout,
            font: asset_server.load(PICO8_FONT),
            draw_state: DrawState::default(),
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
                .get(&self.palette)
                .and_then(|pal| {
                    // Strangely. It's not a 1d texture.
                    match pal.get_color_at(n as u32, 0) {
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
    app
        .init_resource::<Pico8State>()
        .add_api_provider::<LuaScriptHost<N9Args>>(Box::new(Pico8API));
}

fn with_pico8<X>(ctx: &Lua, f: impl FnOnce(&mut Pico8) -> Result<X, Error>) -> Result<X, LuaError> {
    let world = ctx.get_world()?;
    let mut world = world.write();
    let mut system_state: SystemState<Pico8> =
        SystemState::new(&mut world);
    let mut pico8 = system_state.get_mut(&mut world);
    let r = f(&mut pico8);
    system_state.apply(&mut world);
    r.map_err(LuaError::from)
}

impl APIProvider for Pico8API {
    type APITarget = Mutex<Lua>;
    type ScriptContext = Mutex<Lua>;
    type DocTarget = LuaDocFragment;

    fn attach_api(&mut self, ctx: &mut Self::APITarget) -> Result<(), ScriptError> {
        // callbacks can receive any `ToLuaMulti` arguments, here '()' and
        // return any `FromLuaMulti` arguments, here a `usize`
        // check the Rlua documentation for more details

        let ctx = ctx.get_mut().unwrap();
        crate::macros::define_globals! {
            // XXX: This should be demoted in favor of a general `input` solution.
            fn btnp(ctx, b: (Option<u8>)) {
                with_pico8(ctx, |pico8| pico8.btnp(b))
            }

            fn btn(ctx, b: (Option<u8>)) {
                with_pico8(ctx, |pico8| pico8.btnp(b))
            }

            fn cls(ctx, value: (Option<N9Color>)) {
                with_pico8(ctx, |pico8| pico8.cls(value))
            }

            fn pset(ctx, (x, y, color): (u32, u32, Option<N9Color>)) {
                with_pico8(ctx, |pico8| {
                    // We want to ignore out of bounds errors specifically.
                    // Ok(pico8.pset(x, y, color)?)
                    let _ = pico8.pset(x, y, color);
                    Ok(())
                })
            }

            fn rectfill(ctx, (x0, y0, x1, y1, color): (u32, u32, u32, u32, Option<N9Color>)) {
                with_pico8(ctx, |pico8| {
                    // We want to ignore out of bounds errors specifically.
                    // Ok(pico8.pset(x, y, color)?)
                    let _ = pico8.rectfill(UVec2::new(x0, y0), UVec2::new(x1, y1), color);
                    Ok(())
                })
            }

            fn rect(ctx, (x0, y0, x1, y1, color): (u32, u32, u32, u32, Option<N9Color>)) {
                with_pico8(ctx, |pico8| {
                    // We want to ignore out of bounds errors specifically.
                    // Ok(pico8.pset(x, y, color)?)
                    let _ = pico8.rect(UVec2::new(x0, y0), UVec2::new(x1, y1), color);
                    Ok(())
                })
            }

            // spr(n, [x,] [y,] [w,] [h,] [flip_x,] [flip_y])
            // XXX: What's the difference between sprite and spr?
            //
            // Sprite uses N9Entity, which is perhaps more general and dynamic.
            fn spr(ctx, (mut args): LuaMultiValue) {
                let n = args.pop_front().and_then(|v| v.as_usize()).expect("sprite id");
                let spr_args_maybe = if !args.is_empty() {
                    let x = args.pop_front().and_then(|v| v.to_f32()).unwrap_or(0.0);
                    let y = args.pop_front().and_then(|v| v.to_f32()).unwrap_or(0.0);
                    let w = args.pop_front().and_then(|v| v.to_f32());
                    let h = args.pop_front().and_then(|v| v.to_f32());
                    let flip_x = args.pop_front().and_then(|v| v.as_boolean()).unwrap_or(false);
                    let flip_y = args.pop_front().and_then(|v| v.as_boolean()).unwrap_or(false);
                    Some(SprArgs {
                        pos: Vec2::new(x, y),
                        flip_x,
                        flip_y,
                        size: w.or(h).is_some().then(|| Vec2::new(w.unwrap_or(1.0), h.unwrap_or(1.0))),
                    })
                } else {
                    None
                };

                // We get back an entity. Not doing anything with it here yet.
                let _id = with_pico8(ctx, move |pico8| pico8.spr(n, spr_args_maybe))?;
                Ok(())
            }

            // map( celx, cely, sx, sy, celw, celh, [layer] )
            fn map(ctx, (mut args): LuaMultiValue) {
                let celx = args.pop_front().and_then(|v| v.as_u32()).expect("celx");
                let cely = args.pop_front().and_then(|v| v.as_u32()).expect("cely");
                let sx = args.pop_front().and_then(|v| v.to_f32()).expect("sx");
                let sy = args.pop_front().and_then(|v| v.to_f32()).expect("sy");
                let celw = args.pop_front().and_then(|v| v.as_u32()).expect("celw");
                let celh = args.pop_front().and_then(|v| v.as_u32()).expect("celh");
                let layer = args.pop_front().and_then(|v| v.as_u32().map(|v| v as u8)).unwrap_or(0);

                // We get back an entity. Not doing anything with it here yet.
                let _id = with_pico8(ctx, move |pico8| pico8.map(UVec2::new(celx, cely), Vec2::new(sx, sy), UVec2::new(celw, celh), layer))?;
                Ok(())
            }

            fn tostr(ctx, v: Value) {
                let tostring: Function = ctx.globals().get("tostring")?;
                tostring.call::<Value,LuaString>(v)
            }

            // print(text, [x,] [y,] [color])
            fn print(ctx, (mut args): LuaMultiValue) {
                let world = ctx.get_world()?;
                let draw_state = {
                    let world = world.read();
                    let pico8 = world.resource::<Pico8State>();
                    pico8.draw_state.clone()
                };
                let text: String = args.pop_front().map(|v| v.to_string().expect("text")).expect("text");
                // let x = args.pop_front().and_then(|v| v.to_f32());
                // let y = args.pop_front().and_then(|v| v.to_f32());
                let x = args.pop_front().and_then(|v| v.as_u32());
                let y = args.pop_front().and_then(|v| v.as_u32());
                let c = args.pop_front().and_then(|v| v.as_usize()).map(N9Color::Palette);
                let pos = x.map(|x| UVec2::new(x, y.unwrap_or(draw_state.print_cursor.y)));
                with_pico8(ctx, move |pico8| pico8.print(text, pos, c))
            }

            // sfx( n, [channel,] [offset,] [length] )
            fn sfx(ctx, (mut args): LuaMultiValue) {
                let n: u8 = args.pop_front().and_then(|v| v.as_u32()).expect("n") as u8;
                let channel: Option<u8> = args.pop_front().and_then(|v| v.as_u32()).map(|w| w as u8);
                let offset: Option<u8> = args.pop_front().and_then(|v| v.as_u32()).map(|w| w as u8);
                let length: Option<u8> = args.pop_front().and_then(|v| v.as_u32()).map(|w| w as u8);
                with_pico8(ctx, move |pico8| pico8.sfx(n, channel, offset, length))
            }

            fn flr(ctx, v: Number) {
                Ok(v.floor() as u32)
            }

            fn sub(ctx, (string, mut start, end): (LuaString, usize, Option<usize>)) {
                let string = string.to_str()?;
                if start < 0 {
                    start = string.len() - start;
                }
                start -= 1;
                // end -= 1;
                match end {
                    Some(mut end) => {
                        if end < 0 {
                            end = string.len() - end;
                        }
                        if start <= end {
                            Ok(string.chars().skip(start).take(end - start).collect())
                            // BUG: This cuts unicode boundaries.
                            // Ok(string[start..end].to_string())
                        } else {
                            Ok(String::new())
                        }
                    }
                    None => Ok(string.chars().skip(start).collect())
                }
            }

            fn min(ctx, (x, y): (Value, Value)) {
                Ok(if x.to_f32() < y.to_f32() {
                    x
                } else {
                    y
                })
            }

            fn max(ctx, (x, y): (Value, Value)) {
                Ok(if x.to_f32() > y.to_f32() {
                    x
                } else {
                    y
                })
            }

            fn ord(ctx, (string, index, count): (LuaString, Option<usize>, Option<usize>)) {
                let string = string.to_str()?;
                let index = index.map(|i| i - 1).unwrap_or(0);
                let count = count.unwrap_or(1);
                let mut result: Vec<Value> = Vec::with_capacity(count);
                for c in string.chars().skip(index).take(count) {
                    result.push(Value::Integer(c as i64));
                }
                Ok(LuaMultiValue::from_vec(result))
            }
        }

        Ok(())
    }

    fn register_with_app(&self, app: &mut App) {
        // app.register_type::<Settings>();
    }
}

#[cfg(test)]
mod test {


    #[test]
    fn test_suffix_match() {
        let s = "a\\0";
        assert_eq!(s.len(), 3);
        assert!(s.ends_with("\\0"));
    }
}

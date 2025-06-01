mod error;
pub use error::*;
mod asset;
pub use asset::*;
use super::*;
mod spr;
pub use spr::*;
mod state;
pub use state::*;
mod handle;
pub use handle::*;
pub mod input;
use input::*;
mod event;
use event::*;
mod param;
pub use param::*;
mod sfx;
pub use sfx::*;
mod map;
pub use map::*;
mod print;
pub use print::*;
mod rect;
pub use rect::*;
mod circ;
pub use circ::*;
mod oval;
pub use oval::*;
mod pal;
pub use pal::*;
mod bit_ops;
pub use bit_ops::*;

use bevy::{
    audio::PlaybackMode,
    ecs::system::SystemParam,
    image::ImageSampler,
    input::gamepad::GamepadConnectionEvent,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    sprite::Anchor,
    text::TextLayoutInfo,
};
use tiny_skia::{self, FillRule, Paint, PathBuilder, Pixmap, Stroke};

#[cfg(feature = "scripting")]
use bevy_mod_scripting::core::{
        bindings::{function::from::FromScript, script_value::ScriptValue, WorldAccessGuard},
        docgen::typed_through::{ThroughTypeInfo, TypedThrough},
        error::InteropError,
    };

use crate::{
    pico8::{
        self,
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

pub const MAP_COLUMNS: u32 = 128;
pub const PICO8_SPRITE_SIZE: UVec2 = UVec2::new(8, 8);
pub const PICO8_TILE_COUNT: UVec2 = UVec2::new(16, 16);

const ANALOG_STICK_THRESHOLD: f32 = 0.1;

pub(crate) fn plugin(app: &mut App) {
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
        )
        .add_plugins((
            sfx::plugin,
            spr::plugin,
            map::plugin,
            input::plugin,
            print::plugin,
            rect::plugin,
            circ::plugin,
            oval::plugin,
            pal::plugin,
            bit_ops::plugin,
            ))
        ;
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

    // cls([n])
    pub fn cls(&mut self, color: Option<PColor>) -> Result<(), Error> {
        trace!("cls");
        let c = self.get_color(color.unwrap_or(PColor::Palette(0)))?;
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



    pub fn exit(&mut self, error: Option<u8>) {
        self.commands.send_event(match error {
            Some(n) => std::num::NonZero::new(n)
                .map(AppExit::Error)
                .unwrap_or(AppExit::Success),
            None => AppExit::Success,
        });
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


    pub fn color(&mut self, color: Option<PColor>) -> Result<PColor, Error> {
        let last_color = self.state.draw_state.pen;
        if let Some(color) = color {
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
    pub fn stat(&mut self, n: u8, _value: Option<u8>) -> Result<ScriptValue, Error> {
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
    impl super::Pico8<'_, '_> {
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
        let defaults = world.resource::<pico8::Defaults>();
        Pico8State {
            palette: 0,
            pal_map: PalMap::default(),
            draw_state: {
                let mut draw_state = DrawState::default();
                draw_state.pen = PColor::Palette(defaults.pen_color);
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
            palettes: vec![Palette::from_slice(&crate::pico8::PALETTE)],
            border: asset_server.load_with_settings(PICO8_BORDER, pixel_art_settings),
            font: vec![N9Font {
                handle: asset_server.load(PICO8_FONT),
            }],
            audio_banks: Vec::new(),
            sprite_sheets: Vec::new(),
            maps: Vec::new(),
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_suffix_match() {
        let s = "a\\0";
        assert_eq!(s.len(), 3);
        assert!(s.ends_with("\\0"));
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

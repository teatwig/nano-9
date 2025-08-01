use std::f32::consts::PI;

use super::*;

use crate::pico8::Gfx;

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
        Spr::Cur {
            sprite: sprite as usize,
        }
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

impl super::Pico8<'_, '_> {
    /// sspr( sx, sy, sw, sh, dx, dy, [dw,] [dh,] [flip_x,] [flip_y,] [sheet_index])
    pub fn sspr(
        &mut self,
        sprite_rect: Rect,
        screen_pos: Vec2,
        screen_size: Option<Vec2>,
        flip: Option<BVec2>,
        sheet_index: Option<usize>,
    ) -> Result<Entity, Error> {
        let screen_pos = pixel_snap(self.state.draw_state.apply_camera_delta(screen_pos));
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
                        palette,
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

    pub(crate) fn pico8_asset(&self) -> Result<&Pico8Asset, Error> {
        self.pico8_assets
            .get(&self.pico8_handle.handle)
            .ok_or(Error::NoSuch("Pico8Asset".into()))
    }

    pub(crate) fn pico8_asset_mut(&mut self) -> Result<&mut Pico8Asset, Error> {
        self.pico8_assets
            .get_mut(&self.pico8_handle.handle)
            .ok_or(Error::NoSuch("Pico8Asset".into()))
    }

    fn sprite_sheet(&self, sheet_index: Option<usize>) -> Result<&SpriteSheet, Error> {
        let index = sheet_index.unwrap_or(0);
        self.pico8_asset()?
            .sprite_sheets
            .get(index)
            .ok_or(Error::NoSuch(format!("image index {index}").into()))
    }

    fn sprite_sheet_mut(&mut self, sheet_index: Option<usize>) -> Result<&mut SpriteSheet, Error> {
        let index = sheet_index.unwrap_or(0);
        self.pico8_asset_mut()?
            .sprite_sheets
            .get_mut(index)
            .ok_or(Error::NoSuch(format!("image index {index}").into()))
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
        let pos = pixel_snap(self.state.draw_state.apply_camera_delta(pos));
        let x = pos.x;
        let y = pos.y;
        let flip = flip.unwrap_or_default();
        let (sprites, index): (&SpriteSheet, usize) = match spr.into() {
            Spr::Cur { sprite } => (self.sprite_sheet(None)?, sprite),
            Spr::From { sheet, sprite } => (self.sprite_sheet(Some(sheet))?, sprite),
            Spr::Set { sheet: _ } => {
                todo!("sheet set not implemented and maybe shouldn't be");
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
        };
        Ok(())
    }
}

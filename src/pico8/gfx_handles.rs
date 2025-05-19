use bevy::prelude::*;

use crate::pico8::{FillPat, Gfx, PalMap, Palette, Error};

use std::{
    collections::{HashMap, hash_map::Entry},
    hash::{DefaultHasher, Hash, Hasher},
};

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<GfxHandles>().add_systems(
        PostUpdate,
        |mut gfx_handles: ResMut<GfxHandles>| {
            gfx_handles.tick();
        },
    );
}

/// A double-buffered map of (Gfx, PalMap) -> Handle<Image>
///
/// It hands out strong handles and internally persists a strong handle for
/// a tick or frame. This permits the standard drawing scheme of `cls();
/// spr(1)` to not cause asset churn.
///
/// In cases where one is not drawing from the same sprite sheet each frame, one
/// can retain the sprite to avoid churn.
/// ```lua
/// function _init()
///     local s = spr(0):retain()
///     -- Can set visibility to false so it is retained but not drawn.
///     s:vis(false)
///
/// end
/// ```
#[derive(Debug, Resource)]
#[derive(Default)]
pub struct GfxHandles {
    buffers: [HashMap<u64, Handle<Image>>; 2],
    tick: usize,
}


impl GfxHandles {
    /// This returns a strong handle if it was created and caches a weak handle.
    /// Otherwise it returns an extant weak_handle.
    pub fn get_or_create(
        &mut self,
        palette: &Palette,
        pal_map: &PalMap,
        fill_pat: Option<&FillPat>,
        gfx: &Handle<Gfx>,
        gfxs: &Assets<Gfx>,
        images: &mut Assets<Image>,
    ) -> Result<Handle<Image>, Error> {
        let mut hasher = DefaultHasher::new();
        pal_map.hash(&mut hasher);
        if let Some(fill_pat) = fill_pat {
            fill_pat.hash(&mut hasher);
        }
        gfx.hash(&mut hasher);
        let hash = hasher.finish();
        let other_handle: Option<Handle<Image>> = self.buffers[(self.tick + 1) % 2].get(&hash).cloned();
        let map = &mut self.buffers[self.tick % 2];
        let handle: Handle<Image> = match map.entry(hash) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                if let Some(handle) = other_handle {
                    entry.insert(handle).clone()
                } else {
                    let gfx = gfxs.get(gfx).ok_or(Error::NoSuch("gfx asset".into()))?;
                    let image = if let Some(fill_pat) = fill_pat {
                        todo!();
                    } else {
                        gfx.try_to_image(|i, _, bytes| pal_map.write_color(&palette.data, i, bytes))?
                    };
                    entry.insert(images.add(image)).clone()
                }
            }
        };
        Ok(handle)
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        self.buffers[self.tick % 2].clear();
    }
}

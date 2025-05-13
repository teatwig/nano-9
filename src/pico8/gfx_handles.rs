use bevy::{asset::StrongHandle, prelude::*};

use crate::pico8::{FillPat, Gfx, PalMap, Palette};

use std::{
    sync::Arc,
    collections::HashMap,
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

/// A double-buffered map of (Gfx, PalMap) -> AssetId<Image>
///
/// It hands out strong handles and internally persists a strong handle for
/// three ticks or frames. This permits the standard drawing scheme of `cls();
/// spr(1)` to not cause asset churn.
#[derive(Debug, Resource)]
pub struct GfxHandles {
    a: HashMap<u64, Handle<Image>>,
    b: HashMap<u64, Handle<Image>>,
    tick: usize,
}

impl Default for GfxHandles {
    fn default() -> Self {
        GfxHandles {
            a: HashMap::<u64, Handle<Image>>::default(),
            b: HashMap::<u64, Handle<Image>>::default(),
            tick: 0,
        }
    }
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
    ) -> Handle<Image> {
        let mut hasher = DefaultHasher::new();
        pal_map.hash(&mut hasher);
        if let Some(fill_pat) = fill_pat {
            fill_pat.hash(&mut hasher);
        }
        gfx.hash(&mut hasher);
        let hash = hasher.finish();
        let other_handle: Option<Handle<Image>> = if self.tick % 2 == 1 { self.a.get(&hash) } else { self.b.get(&hash) }.cloned();
        let map = if self.tick % 2 == 0 { &mut self.a } else { &mut self.b };
        let handle: &Handle<Image> = map.entry(hash).or_insert_with(|| {
            other_handle.unwrap_or_else(|| {
            let gfx = gfxs.get(gfx).expect("gfx"); //.ok_or(Error::NoSuch("gfx asset".into()))?;
            let image = if let Some(fill_pat) = fill_pat {
                todo!();
            } else {
                gfx.try_to_image(|i, _, bytes| pal_map.write_color(&palette.data, i, bytes))
                    .expect("gfx to image")
            };
            images.add(image)
            })
        });
        handle.clone()
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        if self.tick % 2 == 0 {
            self.a.clear();
        } else {
            self.b.clear();
        }
    }
}

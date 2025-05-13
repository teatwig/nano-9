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

/// A weak map of (Gfx, PalMap) -> AssetId<Image>
///
/// It hands out strong handles and internally persists a strong handle for
/// three ticks or frames. This permits the standard drawing scheme of `cls();
/// spr(1)` to not cause asset churn.
#[derive(Debug, Resource)]
pub struct GfxHandles {
    map: HashMap<u64, AssetId<Image>>,
    tick: usize,
    strong_handles: Vec<Vec<Arc<StrongHandle>>>,
}

impl Default for GfxHandles {
    fn default() -> Self {
        GfxHandles {
            map: HashMap::<u64, AssetId<Image>>::default(),
            tick: 0,
            strong_handles: vec![vec![], vec![], vec![]],
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
        let mut strong_handle = None;
        let handle = *self.map.entry(hash).or_insert_with(|| {
            let gfx = gfxs.get(gfx).expect("gfx"); //.ok_or(Error::NoSuch("gfx asset".into()))?;
            let image = if let Some(fill_pat) = fill_pat {
                todo!();
            } else {
                gfx.try_to_image(|i, _, bytes| pal_map.write_color(&palette.data, i, bytes))
                    .expect("gfx to image")
            };
            let handle = images.add(image);
            let asset_id = handle.id();
            strong_handle = Some(handle);
            asset_id
        });

        if let Some(strong_handle) = strong_handle {
            assert!(strong_handle.is_strong());
            self.push(strong_handle.clone().untyped());
            strong_handle
        } else if let Some(strong_handle) = images.get_strong_handle(handle) {
            assert!(strong_handle.is_strong());
            self.push(strong_handle.clone().untyped());
            strong_handle
        } else {
            self.map.remove(&hash);
            // Will only recurse once.
            self.get_or_create(palette, pal_map, fill_pat, gfx, gfxs, images)
        }
    }

    fn push(&mut self, handle: UntypedHandle) {
        let n = self.strong_handles.len();
        match handle {
            UntypedHandle::Strong(h) => {
                self.strong_handles[self.tick % n].push(h);
            }
            UntypedHandle::Weak(asset_id) => {
                warn!("Cannot persist weak handle {asset_id:?}");
            }
        }
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        let n = self.strong_handles.len();
        // for handle in self.strong_handles[self.tick % n].drain(..) {
        //     drop(handle);
        // }
        self.strong_handles[self.tick % n].clear();
    }
}

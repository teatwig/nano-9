use super::*;

#[derive(Default, Debug, Clone)]
pub enum PalModify {
    #[default]
    Following,
    Present,
    Secondary,
}

impl super::Pico8<'_, '_> {
    pub(crate) fn palette(&self, index: Option<usize>) -> Result<&Palette, Error> {
        self.pico8_asset()?
            .palettes
            .get(index.unwrap_or(self.state.palette))
            .ok_or(Error::NoSuch("palette".into()))
    }

    pub(crate) fn get_color(&self, c: impl Into<N9Color>) -> Result<Color, Error> {
        match c.into().into_pcolor(&self.state.draw_state.pen) {
            PColor::Palette(n) => self.palette(None)?.get_color(n).map(|c| c.into()),
            PColor::Color(c) => Ok(c.into()),
        }
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
}

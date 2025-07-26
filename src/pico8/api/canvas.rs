use super::*;

impl super::Pico8<'_, '_> {
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

    // XXX: pget needed
    // pub fn pget()

    /// Return the size of the canvas
    ///
    /// This is not the window dimensions, which are physical pixels. Instead it
    /// is the number of "logical" pixels, which may be comprised of many
    /// physical pixels.
    pub fn canvas_size(&self) -> UVec2 {
        self.canvas.size
    }
}

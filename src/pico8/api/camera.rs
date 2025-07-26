use super::*;

#[derive(Event, Debug)]
pub(crate) struct UpdateCameraPos(pub(crate) Vec2);

impl super::Pico8<'_, '_> {
    pub fn camera(&mut self, pos: Option<Vec2>) -> Vec2 {
        if let Some(pos) = pos.map(pixel_snap) {
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
}

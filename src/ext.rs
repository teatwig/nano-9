#[cfg(feature = "scripting")]
use bevy_mod_scripting::lua::mlua::Value;

pub trait ValueExt {
    fn to_f32(&self) -> Option<f32>;

    fn to_f32_or<E>(&self, err: E) -> Result<f32, E> {
        self.to_f32().ok_or(err)
    }
}

#[cfg(feature = "scripting")]
impl ValueExt for Value {
    fn to_f32(&self) -> Option<f32> {
        self.as_f32()
            .or_else(|| self.as_integer().map(|x| x as f32))
    }
}

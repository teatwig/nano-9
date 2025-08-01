pub trait ValueExt {
    fn to_f32(&self) -> Option<f32>;

    fn to_f32_or<E>(&self, err: E) -> Result<f32, E> {
        self.to_f32().ok_or(err)
    }
}

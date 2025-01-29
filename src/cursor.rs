use std::ops::{Deref, DerefMut};

/// Keep a `Vec<T>` of `T` and pretend to be `T` with deref magic.
#[derive(Clone, Debug, Default)]
pub struct Cursor<T> {
    pub inner: Vec<T>,
    pub pos: usize,
}

impl<T> From<Vec<T>> for Cursor<T> {
    fn from(v: Vec<T>) -> Self {
        Cursor {
            inner: v,
            pos: 0
        }
    }
}

impl<T> Deref for Cursor<T>{
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner[self.pos]
    }
}

impl<T> DerefMut for Cursor<T>{
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner[self.pos]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn pretend_ive_got_one_item() {
        let mut a: Cursor<u8> = vec![1,2,3].into();
        assert_eq!(*a, 1);
        a.pos += 1;
        assert_eq!(*a, 2);
        *a = 5;
        assert_eq!(a.inner, vec![1,5,3]);
    }
}

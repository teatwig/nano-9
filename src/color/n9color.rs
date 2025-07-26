use super::PColor;
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, Reflect, Default)]
pub enum N9Color {
    #[default]
    Pen,
    PColor(PColor),
}

impl N9Color {
    pub fn into_pcolor(&self, pen_color: &PColor) -> PColor {
        match self {
            N9Color::Pen => *pen_color,
            N9Color::PColor(p) => *p,
        }
    }
}

impl From<PColor> for N9Color {
    fn from(c: PColor) -> Self {
        N9Color::PColor(c)
    }
}

impl From<usize> for N9Color {
    fn from(n: usize) -> Self {
        N9Color::PColor(n.into())
    }
}

impl From<Option<usize>> for N9Color {
    fn from(c: Option<usize>) -> Self {
        match c {
            Some(index) => N9Color::PColor(index.into()),
            None => N9Color::Pen,
        }
    }
}

impl From<Color> for N9Color {
    fn from(c: Color) -> Self {
        N9Color::PColor(c.into())
    }
}

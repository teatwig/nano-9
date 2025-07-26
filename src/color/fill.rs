use bevy::prelude::*;

use super::PColor;

/// This is a fill color that specifies what color to use for the "off" bit (default) and "on" bit.
#[derive(Debug, Clone, Copy, Reflect)]
pub enum FillColor {
    One { off: PColor },
    Two { off: PColor, on: PColor },
}

impl FillColor {
    pub fn on(&self) -> Option<PColor> {
        match self {
            FillColor::One { off: _ } => None,
            FillColor::Two { off: _, on } => Some(*on),
        }
    }

    pub fn off(&self) -> PColor {
        match self {
            FillColor::One { off } => *off,
            FillColor::Two { off, on: _ } => *off,
        }
    }
}

impl From<PColor> for FillColor {
    fn from(c: PColor) -> Self {
        FillColor::One { off: c }
    }
}

impl From<usize> for FillColor {
    fn from(c: usize) -> Self {
        FillColor::One {
            off: PColor::from(c),
        }
    }
}

impl From<Color> for FillColor {
    fn from(c: Color) -> Self {
        FillColor::One {
            off: PColor::Color(c.into()),
        }
    }
}

impl From<(Color, Color)> for FillColor {
    fn from((a, b): (Color, Color)) -> Self {
        FillColor::Two {
            off: b.into(),
            on: a.into(),
        }
    }
}

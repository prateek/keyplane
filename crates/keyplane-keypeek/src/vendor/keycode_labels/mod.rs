//! QMK/VIA numeric keycode → structured label tables, vendored from KeyPeek
//! (`src/qmk_keycode_labels/`). These map raw VIA `u16` keycodes to a
//! [`LayoutKey`](crate::vendor::layout_key::LayoutKey); the Keyplane bridge then
//! converts that into a Semantic Action + Display Legend.

mod advanced;
mod basic;
mod constants;
mod keycode_label;
mod layer;

pub use keycode_label::get_layout_key;

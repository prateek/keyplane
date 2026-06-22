//! Text stand-ins for the egui icon-font glyphs KeyPeek uses for media, mouse,
//! and arrow keycaps. Keyplane renders legends in a web frontend, so the
//! vendored tables reference these short text labels instead of Private-Use-Area
//! font glyphs. Names mirror `egui_phosphor::regular::*` so the table code is
//! otherwise unchanged.

pub const ARROW_UP: &str = "↑";
pub const ARROW_DOWN: &str = "↓";
pub const ARROW_LEFT: &str = "←";
pub const ARROW_RIGHT: &str = "→";
pub const ARROWS_LEFT_RIGHT: &str = "↔";
pub const ARROW_FAT_LINE_UP: &str = "⇧";
pub const ARROW_ELBOW_DOWN_LEFT: &str = "↵";
pub const BACKSPACE: &str = "⌫";

pub const SPEAKER_X: &str = "Mute";
pub const SPEAKER_LOW: &str = "Vol-";
pub const SPEAKER_HIGH: &str = "Vol+";

pub const SUN_DIM: &str = "Bright-";
pub const SUN: &str = "Bright+";

pub const PLAY_PAUSE: &str = "⏯";
pub const STOP: &str = "⏹";
pub const SKIP_FORWARD: &str = "⏭";
pub const SKIP_BACK: &str = "⏮";
pub const REWIND: &str = "⏪";
pub const FAST_FORWARD: &str = "⏩";

pub const POWER: &str = "Power";
pub const LIST: &str = "Menu";

pub const MOUSE_SIMPLE: &str = "Mouse";
pub const MOUSE_SCROLL: &str = "Scroll";
pub const MOUSE_LEFT_CLICK: &str = "LMB";
pub const MOUSE_RIGHT_CLICK: &str = "RMB";
pub const MOUSE_MIDDLE_CLICK: &str = "MMB";

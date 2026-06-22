//! Per-key coordinate geometry (ADR 0028).
//!
//! The canonical Physical Layout uses per-key coordinate geometry, not row
//! arrays, so split, angled, and non-rectangular boards render correctly. Units
//! are keycap units `u` (1u = one standard key width), matching Vial/KLE-style
//! conventions; the renderer scales `u` to pixels.

use serde::{Deserialize, Serialize};

/// The hardware row/column identity for a Physical Key, when the source exposes
/// matrix coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixPosition {
    pub row: u16,
    pub col: u16,
}

impl MatrixPosition {
    pub fn new(row: u16, col: u16) -> Self {
        Self { row, col }
    }
}

/// Coordinate geometry for a Physical Key, in keycap units.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyGeometry {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Rotation in degrees, clockwise, about [`rotation_origin`](Self::rotation_origin).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rotation: f64,
    /// Rotation origin in keycap units. Defaults to the key's own `(x, y)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_origin: Option<(f64, f64)>,
}

fn is_zero(value: &f64) -> bool {
    *value == 0.0
}

impl KeyGeometry {
    /// A 1u-by-1u key at `(x, y)` with no rotation.
    pub fn unit(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            w: 1.0,
            h: 1.0,
            rotation: 0.0,
            rotation_origin: None,
        }
    }

    pub fn sized(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self {
            x,
            y,
            w,
            h,
            rotation: 0.0,
            rotation_origin: None,
        }
    }

    pub fn with_rotation(mut self, degrees: f64, origin: (f64, f64)) -> Self {
        self.rotation = degrees;
        self.rotation_origin = Some(origin);
        self
    }
}

pub mod kle_parser;
pub mod layout_geometry;
pub mod qmk_json_parser;
pub mod via;
pub mod vial;
pub mod zmk;
pub mod zmk_rpc;

use crate::vendor::layout_key::LayoutKey;
use qmk_via_api::api::{KeyboardApi, MatrixInfo};
use std::error::Error;
use std::sync::Arc;

use self::zmk::ZmkProtocol;
pub use self::zmk_rpc::{ZmkTransport, ZmkData, DeviceLocked};

/// Shared raw-keycode reader used by the VIA and Vial protocol impls.
pub(crate) fn read_all_raw_via_api(
    api: &KeyboardApi,
    layers: usize,
    rows: usize,
    cols: usize,
) -> Vec<Vec<Vec<u16>>> {
    let mut codes = vec![vec![vec![0u16; cols]; rows]; layers];
    let matrix_info = MatrixInfo {
        rows: rows as u8,
        cols: cols as u8,
    };
    for (layer, layer_codes) in codes.iter_mut().enumerate().take(layers) {
        if let Ok(raw_matrix) = api.read_raw_matrix(matrix_info, layer as u8) {
            for (i, &keycode) in raw_matrix.iter().enumerate() {
                layer_codes[i / cols][i % cols] = keycode;
            }
        }
    }
    codes
}

use self::via::ViaProtocol;
use self::vial::VialProtocol;

pub const KEYPEEK_SUBSCRIBE_MARKER: u8 = 0xC0;
pub const KEYPEEK_SUBSCRIBE_ACTIVE: u8 = 0xA1;
pub const KEYPEEK_SUBSCRIBE_INACTIVE: u8 = 0xA0;

pub type Row = usize;
pub type Column = usize;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Key {
    pub row: Row,
    pub col: Column,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Rotation angle in degrees, clockwise around the key's center.
    #[serde(default)]
    pub r: f32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyboardLayout {
    pub name: String,
    pub keys: Vec<Key>,
}

impl KeyboardLayout {
    pub fn get_dimensions(&self) -> (f32, f32) {
        let max_x = self.keys.iter().map(|k| k.x + k.w).fold(0.0, f32::max);
        let max_y = self.keys.iter().map(|k| k.y + k.h).fold(0.0, f32::max);
        (max_x, max_y)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyboardDefinition {
    pub vid: u16,
    pub pid: u16,
    pub rows: usize,
    pub cols: usize,
    pub layouts: Vec<KeyboardLayout>,
}

impl KeyboardDefinition {
    pub fn get_layout_names(&self) -> Vec<String> {
        self.layouts.iter().map(|l| l.name.clone()).collect()
    }

    pub fn get_layout(&self, layout_name: &str) -> Result<KeyboardLayout, String> {
        self.layouts
            .iter()
            .find(|l| l.name == layout_name)
            .cloned()
            .ok_or_else(|| format!("Layout '{}' not found.", layout_name))
    }
}

pub trait KeyboardProtocol: Send {
    fn get_layout_definition(&self) -> &KeyboardDefinition;

    fn get_layer_count(&self) -> Result<usize, Box<dyn Error>>;

    fn read_all_keys(
        &self,
        layers: usize,
        rows: usize,
        cols: usize,
    ) -> Vec<Vec<Vec<Option<LayoutKey>>>>;

    /// Read raw VIA `u16` keycodes for every layer position. Keyplane keeps the
    /// raw codes (ADR 0005) and derives Semantic Actions itself, rather than
    /// consuming KeyPeek's pre-labeled `LayoutKey`.
    fn read_all_raw(&self, layers: usize, rows: usize, cols: usize) -> Vec<Vec<Vec<u16>>>;

    fn hid_read(&self) -> Result<Vec<u8>, Box<dyn Error>>;

    fn subscription_sender(&self) -> Option<Box<dyn SubscriptionSender>> {
        None
    }

    fn reopener(&self) -> Option<Arc<dyn Reopener>> {
        None
    }
}

pub trait Reopener: Send + Sync {
    fn reopen(&self) -> Result<Box<dyn KeyboardProtocol>, Box<dyn Error>>;
}

pub trait SubscriptionSender: Send {
    fn set_active(&self, active: bool) -> Result<(), Box<dyn Error>>;
}

pub struct RawHidSubscription {
    api: KeyboardApi,
}

impl RawHidSubscription {
    pub fn open(vid: u16, pid: u16) -> Option<Box<dyn SubscriptionSender>> {
        KeyboardApi::new(vid, pid, 0xff60, None)
            .ok()
            .map(|api| Box::new(Self { api }) as Box<dyn SubscriptionSender>)
    }
}

impl SubscriptionSender for RawHidSubscription {
    fn set_active(&self, active: bool) -> Result<(), Box<dyn Error>> {
        let value = if active {
            KEYPEEK_SUBSCRIBE_ACTIVE
        } else {
            KEYPEEK_SUBSCRIBE_INACTIVE
        };
        self.api
            .hid_send(vec![KEYPEEK_SUBSCRIBE_MARKER, value])
            .map_err(|e| format!("Subscription keepalive write error: {e}").into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZmkTransportConfig {
    Serial(String),
    Ble(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionSpec {
    Via {
        json_path: String,
    },
    Vial {
        vid: u16,
        pid: u16,
    },
    Zmk {
        vid: u16,
        pid: u16,
        transport: ZmkTransportConfig,
    },
}

pub fn connect_protocol(
    spec: &ConnectionSpec,
) -> Result<Box<dyn KeyboardProtocol>, Box<dyn Error>> {
    match spec {
        ConnectionSpec::Via { json_path } => {
            let protocol = ViaProtocol::connect(json_path)?;
            Ok(Box::new(protocol))
        }
        ConnectionSpec::Vial { vid, pid } => {
            let protocol = VialProtocol::connect(*vid, *pid)?;
            Ok(Box::new(protocol))
        }
        ConnectionSpec::Zmk {
            vid,
            pid,
            transport,
        } => {
            let zmk_transport = match transport {
                ZmkTransportConfig::Serial(port) => ZmkTransport::SerialPort(port.clone()),
                ZmkTransportConfig::Ble(id) => ZmkTransport::BleDevice(id.clone()),
            };
            let protocol = ZmkProtocol::connect_live(*vid, *pid, &zmk_transport)?;
            Ok(Box::new(protocol))
        }
    }
}

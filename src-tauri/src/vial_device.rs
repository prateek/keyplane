use crate::importers::{import_vial_device_snapshot, vial_matrix_dimensions, VialDeviceSnapshot};
use qmk_via_api::api::{KeyboardApi, MatrixInfo};
use serde_json::Value as JsonValue;
use std::io::Cursor;
use thiserror::Error;

pub const VIAL_PREFIX: u8 = 0xfe;
const VIAL_KEYBOARD_ID_COMMAND: u8 = 0x00;
const VIAL_DEFINITION_SIZE_COMMAND: u8 = 0x01;
const VIAL_DEFINITION_BLOCK_COMMAND: u8 = 0x02;
const RAW_HID_USAGE_PAGE: u16 = 0xff60;
const RAW_HID_REPORT_SIZE: usize = 32;

#[derive(Debug, Error)]
pub enum VialDeviceError {
    #[error("{0}")]
    Transport(String),
    #[error("{0}")]
    Protocol(String),
    #[error("Vial definition decode failed: {0}")]
    Decode(String),
    #[error("Vial device import failed: {0}")]
    Import(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VialDeviceIdentity {
    pub protocol_version: u32,
    pub uid: String,
}

pub trait VialDeviceTransport {
    fn write_report(&mut self, report: &[u8]) -> Result<(), VialDeviceError>;
    fn read_report(&mut self) -> Result<Vec<u8>, VialDeviceError>;
    fn layer_count(&mut self) -> Result<u8, VialDeviceError>;
    fn read_raw_matrix(
        &mut self,
        rows: u8,
        cols: u8,
        layer: u8,
    ) -> Result<Vec<u16>, VialDeviceError>;
}

pub struct QmkViaVialTransport {
    api: KeyboardApi,
}

impl QmkViaVialTransport {
    pub fn open(vid: u16, pid: u16) -> Result<Self, VialDeviceError> {
        KeyboardApi::new(vid, pid, RAW_HID_USAGE_PAGE, None)
            .map(|api| Self { api })
            .map_err(|err| VialDeviceError::Transport(format!("HID open failed: {err}")))
    }
}

impl VialDeviceTransport for QmkViaVialTransport {
    fn write_report(&mut self, report: &[u8]) -> Result<(), VialDeviceError> {
        self.api
            .hid_send(report.to_vec())
            .map_err(|err| VialDeviceError::Transport(format!("HID write failed: {err}")))
    }

    fn read_report(&mut self) -> Result<Vec<u8>, VialDeviceError> {
        self.api
            .hid_read()
            .map_err(|err| VialDeviceError::Transport(format!("HID read failed: {err}")))
    }

    fn layer_count(&mut self) -> Result<u8, VialDeviceError> {
        self.api
            .get_layer_count()
            .map_err(|err| VialDeviceError::Transport(format!("Layer count read failed: {err}")))
    }

    fn read_raw_matrix(
        &mut self,
        rows: u8,
        cols: u8,
        layer: u8,
    ) -> Result<Vec<u16>, VialDeviceError> {
        self.api
            .read_raw_matrix(MatrixInfo { rows, cols }, layer)
            .map_err(|err| {
                VialDeviceError::Transport(format!("Layer {layer} raw matrix read failed: {err}"))
            })
    }
}

pub fn import_vial_device<T: VialDeviceTransport>(
    transport: &mut T,
    vid: u16,
    pid: u16,
) -> Result<crate::domain::ImportCandidate, VialDeviceError> {
    let identity = read_vial_identity(transport)?;
    if identity.protocol_version == 0 {
        return Err(VialDeviceError::Protocol(
            "Device does not support the Vial protocol".to_string(),
        ));
    }
    let definition_json = read_vial_definition_json(transport)?;
    let (rows, cols) = vial_matrix_dimensions(&definition_json).ok_or_else(|| {
        VialDeviceError::Protocol(
            "Vial definition did not expose matrix-addressed layouts.keymap geometry".to_string(),
        )
    })?;
    let rows = u8::try_from(rows)
        .map_err(|_| VialDeviceError::Protocol(format!("Vial matrix has too many rows: {rows}")))?;
    let cols = u8::try_from(cols)
        .map_err(|_| VialDeviceError::Protocol(format!("Vial matrix has too many cols: {cols}")))?;
    let layer_count = transport.layer_count()?;
    if layer_count == 0 {
        return Err(VialDeviceError::Protocol(
            "Vial device reported zero layers".to_string(),
        ));
    }

    let mut raw_matrices = Vec::new();
    for layer in 0..layer_count {
        raw_matrices.push(raw_matrix_rows(
            transport.read_raw_matrix(rows, cols, layer)?,
            rows,
            cols,
        )?);
    }

    import_vial_device_snapshot(VialDeviceSnapshot {
        vid,
        pid,
        uid: identity.uid,
        protocol_version: identity.protocol_version,
        definition_json,
        raw_matrices,
    })
    .map_err(|err| VialDeviceError::Import(err.to_string()))
}

fn read_vial_identity<T: VialDeviceTransport>(
    transport: &mut T,
) -> Result<VialDeviceIdentity, VialDeviceError> {
    write_vial_command(transport, VIAL_KEYBOARD_ID_COMMAND, &[])?;
    let response = transport.read_report()?;
    if response.len() < 12 {
        return Err(VialDeviceError::Protocol(
            "Vial keyboard-id response was shorter than 12 bytes".to_string(),
        ));
    }

    let protocol_version = u32::from_le_bytes([response[0], response[1], response[2], response[3]]);
    let uid_bytes: [u8; 8] = response[4..12]
        .try_into()
        .map_err(|_| VialDeviceError::Protocol("Vial UID response was malformed".to_string()))?;
    Ok(VialDeviceIdentity {
        protocol_version,
        uid: format!("{:016x}", u64::from_le_bytes(uid_bytes)),
    })
}

fn read_vial_definition_json<T: VialDeviceTransport>(
    transport: &mut T,
) -> Result<JsonValue, VialDeviceError> {
    let size = read_vial_definition_size(transport)?;
    if size == 0 {
        return Err(VialDeviceError::Protocol(
            "Vial definition size is 0".to_string(),
        ));
    }

    let mut compressed = Vec::with_capacity(size);
    let mut block = 0_u32;
    while compressed.len() < size {
        let response = read_vial_definition_block(transport, block)?;
        let remaining = size - compressed.len();
        let chunk_size = remaining.min(RAW_HID_REPORT_SIZE).min(response.len());
        if chunk_size == 0 {
            return Err(VialDeviceError::Protocol(
                "Vial definition block response was empty".to_string(),
            ));
        }
        compressed.extend_from_slice(&response[..chunk_size]);
        block = block.saturating_add(1);
    }

    let mut decompressed = Vec::new();
    lzma_rs::xz_decompress(&mut Cursor::new(&compressed), &mut decompressed)
        .map_err(|err| VialDeviceError::Decode(err.to_string()))?;
    serde_json::from_slice(&decompressed).map_err(|err| VialDeviceError::Decode(err.to_string()))
}

fn read_vial_definition_size<T: VialDeviceTransport>(
    transport: &mut T,
) -> Result<usize, VialDeviceError> {
    write_vial_command(transport, VIAL_DEFINITION_SIZE_COMMAND, &[])?;
    let response = transport.read_report()?;
    if response.len() < 4 {
        return Err(VialDeviceError::Protocol(
            "Vial definition-size response was shorter than 4 bytes".to_string(),
        ));
    }
    Ok(u32::from_le_bytes([response[0], response[1], response[2], response[3]]) as usize)
}

fn read_vial_definition_block<T: VialDeviceTransport>(
    transport: &mut T,
    block: u32,
) -> Result<Vec<u8>, VialDeviceError> {
    let mut payload = [0_u8; 4];
    payload.copy_from_slice(&block.to_le_bytes());
    write_vial_command(transport, VIAL_DEFINITION_BLOCK_COMMAND, &payload)?;
    transport.read_report()
}

fn write_vial_command<T: VialDeviceTransport>(
    transport: &mut T,
    command: u8,
    payload: &[u8],
) -> Result<(), VialDeviceError> {
    let mut report = vec![0_u8; RAW_HID_REPORT_SIZE];
    report[0] = VIAL_PREFIX;
    report[1] = command;
    let copy_len = payload.len().min(RAW_HID_REPORT_SIZE - 2);
    report[2..2 + copy_len].copy_from_slice(&payload[..copy_len]);
    transport.write_report(&report)
}

fn raw_matrix_rows(
    flattened: Vec<u16>,
    rows: u8,
    cols: u8,
) -> Result<Vec<Vec<u16>>, VialDeviceError> {
    let rows = usize::from(rows);
    let cols = usize::from(cols);
    let expected = rows * cols;
    if flattened.len() != expected {
        return Err(VialDeviceError::Protocol(format!(
            "Vial raw matrix length mismatch: expected {expected}, got {}",
            flattened.len()
        )));
    }

    Ok(flattened
        .chunks(cols)
        .map(|row| row.to_vec())
        .collect::<Vec<_>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[derive(Default)]
    struct FakeVialTransport {
        writes: Vec<Vec<u8>>,
        reads: VecDeque<Vec<u8>>,
        layer_count: u8,
        matrices: Vec<Vec<u16>>,
    }

    impl VialDeviceTransport for FakeVialTransport {
        fn write_report(&mut self, report: &[u8]) -> Result<(), VialDeviceError> {
            self.writes.push(report.to_vec());
            Ok(())
        }

        fn read_report(&mut self) -> Result<Vec<u8>, VialDeviceError> {
            self.reads
                .pop_front()
                .ok_or_else(|| VialDeviceError::Transport("fake read queue is empty".to_string()))
        }

        fn layer_count(&mut self) -> Result<u8, VialDeviceError> {
            Ok(self.layer_count)
        }

        fn read_raw_matrix(
            &mut self,
            _rows: u8,
            _cols: u8,
            layer: u8,
        ) -> Result<Vec<u16>, VialDeviceError> {
            self.matrices
                .get(usize::from(layer))
                .cloned()
                .ok_or_else(|| {
                    VialDeviceError::Transport(format!("fake matrix {layer} is missing"))
                })
        }
    }

    fn compressed_definition(json: &str) -> Vec<u8> {
        let mut compressed = Vec::new();
        lzma_rs::xz_compress(&mut Cursor::new(json.as_bytes()), &mut compressed)
            .expect("definition compresses");
        compressed
    }

    fn padded_report(bytes: &[u8]) -> Vec<u8> {
        let mut report = vec![0_u8; RAW_HID_REPORT_SIZE];
        let len = bytes.len().min(RAW_HID_REPORT_SIZE);
        report[..len].copy_from_slice(&bytes[..len]);
        report
    }

    #[test]
    fn imports_vial_device_definition_and_raw_matrices_through_fake_transport() {
        let definition = r#"{"layouts":{"keymap":[["0,0","0,1"]]}}"#;
        let compressed = compressed_definition(definition);
        let mut size = vec![0_u8; RAW_HID_REPORT_SIZE];
        size[..4].copy_from_slice(&(compressed.len() as u32).to_le_bytes());
        let mut identity = vec![0_u8; RAW_HID_REPORT_SIZE];
        identity[..4].copy_from_slice(&6_u32.to_le_bytes());
        identity[4..12].copy_from_slice(&0x1122_3344_5566_7788_u64.to_le_bytes());
        let mut reads = VecDeque::new();
        reads.push_back(identity);
        reads.push_back(size);
        for chunk in compressed.chunks(RAW_HID_REPORT_SIZE) {
            reads.push_back(padded_report(chunk));
        }
        let mut transport = FakeVialTransport {
            writes: Vec::new(),
            reads,
            layer_count: 2,
            matrices: vec![vec![0x0004, 0x0005], vec![0x0001, 0x0000]],
        };

        let candidate =
            import_vial_device(&mut transport, 0xfeed, 0xcafe).expect("fake Vial device imports");

        assert_eq!(candidate.source.kind, "vial-device-import");
        assert!(candidate.best_effort_preview);
        assert_eq!(candidate.summary.imported_keys, 2);
        assert_eq!(candidate.summary.imported_layers, 2);
        assert_eq!(
            candidate.preview_profile.runtime_backends[0].capabilities,
            vec![
                crate::domain::CapabilityFlag::ImportGeometry,
                crate::domain::CapabilityFlag::ImportKeymaps,
                crate::domain::CapabilityFlag::PreviewOnly,
            ]
        );
        assert_eq!(
            candidate.preview_profile.keymap.layers[0].actions[0]
                .raw
                .value,
            "KC_A"
        );
        assert_eq!(
            candidate.preview_profile.keymap.layers[1].actions[0]
                .raw
                .value,
            "KC_TRNS"
        );
        assert_eq!(
            transport.writes[0][..2],
            [VIAL_PREFIX, VIAL_KEYBOARD_ID_COMMAND]
        );
        assert_eq!(
            transport.writes[1][..2],
            [VIAL_PREFIX, VIAL_DEFINITION_SIZE_COMMAND]
        );
        assert_eq!(
            transport.writes[2][..6],
            [VIAL_PREFIX, VIAL_DEFINITION_BLOCK_COMMAND, 0, 0, 0, 0]
        );
    }

    #[test]
    fn rejects_malformed_raw_matrix_lengths() {
        let error = raw_matrix_rows(vec![1, 2, 3], 2, 2).expect_err("length mismatch fails");

        assert!(
            matches!(error, VialDeviceError::Protocol(message) if message.contains("expected 4"))
        );
    }
}

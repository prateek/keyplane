//! List KeyPeek-discoverable keyboards (VIA/Vial/ZMK over HID, plus ZMK
//! serial/BLE), using KeyPeek's device-discovery logic. Run:
//!   cargo run -p keyplane-keypeek --example discover
//!
//! The classification/labelling logic is unit-tested (the vendored
//! `device_discovery` tests, ported from KeyPeek). This example exercises it
//! against the host's real HID devices.

use keyplane_keypeek::discover_devices;

fn main() {
    let devices = discover_devices();
    if devices.is_empty() {
        println!("no KeyPeek-supported devices found (none attached?)");
        return;
    }
    for d in devices {
        println!(
            "{}  vid={:04x} pid={:04x} kind={}{}{}",
            d.display_name(),
            d.vid,
            d.pid,
            d.kind.label(),
            d.serial_port.map(|p| format!(" serial={p}")).unwrap_or_default(),
            d.ble_device_id.map(|b| format!(" ble={b}")).unwrap_or_default(),
        );
    }
}

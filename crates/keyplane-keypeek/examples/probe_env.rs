//! Probe the real environment: enumerate HID devices and report the macOS
//! Input Monitoring permission. Run: cargo run -p keyplane-keypeek --example probe_env
use hidapi::HidApi;

fn main() {
    match HidApi::new() {
        Ok(api) => {
            let mut count = 0;
            let mut keyboards = 0;
            for d in api.device_list() {
                count += 1;
                let up = d.usage_page();
                let usage = d.usage();
                // VIA/Vial/QMK raw HID = usage_page 0xff60; keyboards = 0x0001/0x0006
                let is_kbd = up == 0x0001 && usage == 0x0006;
                let is_raw = up == 0xff60;
                if is_kbd || is_raw {
                    keyboards += 1;
                    println!(
                        "  {:04x}:{:04x} up=0x{:04x} usage=0x{:02x} {} {} {}",
                        d.vendor_id(),
                        d.product_id(),
                        up,
                        usage,
                        d.manufacturer_string().unwrap_or("?"),
                        d.product_string().unwrap_or("?"),
                        if is_raw { "[RAW-HID/VIA]" } else { "[keyboard]" }
                    );
                }
            }
            println!("total HID interfaces: {count}; keyboard/raw-hid interfaces: {keyboards}");
        }
        Err(e) => println!("hidapi init failed: {e}"),
    }
}

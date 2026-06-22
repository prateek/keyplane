//! Probe the real environment: enumerate every HID device and try a
//! non-blocking read to exercise the hidapi transport against whatever real
//! hardware is present. Run: cargo run -p keyplane-keypeek --example probe_env
use hidapi::HidApi;

fn main() {
    let api = match HidApi::new() {
        Ok(a) => a,
        Err(e) => { println!("hidapi init failed: {e}"); return; }
    };
    let devices: Vec<_> = api.device_list().collect();
    println!("HID interfaces: {}", devices.len());
    for d in &devices {
        println!(
            "  {:04x}:{:04x} up=0x{:04x} usage=0x{:02x} mfg={:?} product={:?}",
            d.vendor_id(), d.product_id(), d.usage_page(), d.usage(),
            d.manufacturer_string(), d.product_string()
        );
    }
    let keyboards = devices.iter().filter(|d| d.usage_page()==0x0001 && d.usage()==0x0006).count();
    let raw = devices.iter().filter(|d| d.usage_page()==0xff60).count();
    println!("keyboard interfaces: {keyboards}; VIA raw-hid interfaces: {raw}");

    // Exercise the transport: open the first device and attempt a timed read.
    if let Some(d) = devices.first() {
        match d.open_device(&api) {
            Ok(dev) => {
                let mut buf = [0u8; 64];
                let _ = dev.set_blocking_mode(false);
                match dev.read_timeout(&mut buf, 100) {
                    Ok(n) => println!("transport OK: opened a real HID device and read {n} bytes"),
                    Err(e) => println!("transport: opened device, read returned: {e}"),
                }
            }
            Err(e) => println!("transport: open failed (often needs permission): {e}"),
        }
    }
}

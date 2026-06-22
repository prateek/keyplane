fn main() {
    match keyplane_lib::keypeek_live::discover_keypeek_devices() {
        Ok(devices) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&devices).expect("discovered devices serialize")
            );
        }
        Err(err) => {
            eprintln!("Could not discover KeyPeek-compatible VIA Raw HID devices: {err}");
            std::process::exit(1);
        }
    }
}

use std::collections::HashMap;

include!(concat!(env!("OUT_DIR"), "/generated-console.rs"));

pub fn console_assets() -> HashMap<&'static str, ::static_files::Resource> {
    generate_console_assets()
}

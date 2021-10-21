#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings), windows_subsystem = "windows")] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod program;
mod tree;
mod util;

fn main() {
    let app = app::App::default();
    let native_options = eframe::NativeOptions {
        icon_data: Some(eframe::epi::IconData {
            height: 48,
            width: 48,
            rgba: include_bytes!("../data/plasticity.rgb").to_vec(),
        }),
        ..Default::default()
    };
    eframe::run_native(Box::new(app), native_options);
}

mod animals;
mod assets;
mod audio;
mod effects;
mod flow;
mod instruments;
mod ui;

use cpal::traits::{DeviceTrait, HostTrait};

fn main() {
    let _viewer = jungle_viewer::JungleViewerBuilder::new().title("Welcome Example");

    let host = cpal::default_host();
    let device_name = host
        .default_output_device()
        .and_then(|device| device.description().ok().map(|description| description.name().to_string()))
        .unwrap_or_else(|| "none".to_string());

    println!("welcome example");
    println!("default audio output device: {device_name}");
}

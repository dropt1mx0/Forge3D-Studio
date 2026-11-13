#![allow(clippy::collapsible_if)]

use eframe::egui;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use forge3d_studio::Forge3dStudioApp;

fn load_window_icon() -> Option<egui::IconData> {
    let decoded = image::load_from_memory(include_bytes!("../assets/forge3d-logo.jpg")).ok()?;
    let icon = decoded
        .resize_exact(64, 64, image::imageops::FilterType::Lanczos3)
        .to_rgba8();

    Some(egui::IconData {
        width: icon.width(),
        height: icon.height(),
        rgba: icon.into_raw(),
    })
}

fn main() -> eframe::Result<()> {
    // 1. Initialize structured logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "forge3d_studio=info,forge_render=info,forge_gpu=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Forge3D Studio — Alpha 0.1");

    // 2. Configure native application window and WGPU options
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Forge3D Studio")
        .with_app_id("forge3d-studio")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([800.0, 600.0]);
    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(icon);
    } else {
        tracing::warn!("Could not decode the embedded Forge3D window icon");
    }

    let native_options = eframe::NativeOptions {
        viewport,
        wgpu_options: egui_wgpu::WgpuConfiguration::default(),
        ..Default::default()
    };

    // 3. Launch native event loop
    eframe::run_native(
        "Forge3D Studio",
        native_options,
        Box::new(|cc| Ok(Box::new(Forge3dStudioApp::new(cc)))),
    )
}

use eframe::egui;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use forge3d_studio::Forge3dStudioApp;

fn main() -> eframe::Result<()> {
    // 1. Initialize structured logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "forge3d_studio=info,forge_render=info,forge_gpu=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Forge3D Rust Edition — Alpha 0.1");

    // 2. Configure native application window and WGPU options
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Forge3D Rust Edition — Alpha 0.1")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_decorations(false),
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

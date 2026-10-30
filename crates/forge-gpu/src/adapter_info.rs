use std::fmt;

/// Formatted metadata about the active GPU adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuAdapterInfo {
    pub name: String,
    pub backend: String,
    pub device_type: String,
    pub driver: String,
    pub driver_info: String,
}

impl GpuAdapterInfo {
    pub fn from_wgpu(info: &wgpu::AdapterInfo) -> Self {
        Self {
            name: info.name.clone(),
            backend: format!("{:?}", info.backend),
            device_type: format!("{:?}", info.device_type),
            driver: info.driver.clone(),
            driver_info: info.driver_info.clone(),
        }
    }

    /// Short one-line summary for the status bar.
    pub fn short_summary(&self) -> String {
        format!("{} ({})", self.name, self.backend)
    }
}

impl fmt::Display for GpuAdapterInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Adapter: {} | Backend: {} | Type: {} | Driver: {}",
            self.name, self.backend, self.device_type, self.driver
        )
    }
}

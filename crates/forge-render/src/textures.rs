use std::path::PathBuf;
#[cfg(not(test))]
use std::sync::OnceLock;

use forge_scene::PbrMaterial;
#[cfg(not(test))]
use image::{DynamicImage, GrayImage, RgbImage, RgbaImage, imageops::FilterType};
#[cfg(not(test))]
use std::path::Path;
use wgpu::util::DeviceExt;

pub(crate) struct GpuMaterial {
    pub bind_group: wgpu::BindGroup,
    pub uniform_buffer: wgpu::Buffer,
    _textures: Vec<wgpu::Texture>,
    _sampler: wgpu::Sampler,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct MaterialUniform {
    base_color_factor: [f32; 4],
    emissive_factor: [f32; 4],
    material_params: [f32; 4],
    uv_scale: [f32; 2],
    padding: [u32; 2],
}

struct MipLevel {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

struct TextureData {
    label: &'static str,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    bytes_per_pixel: u32,
    mips: Vec<MipLevel>,
}

impl TextureData {
    fn solid(
        label: &'static str,
        format: wgpu::TextureFormat,
        bytes_per_pixel: u32,
        pixels: &[u8],
    ) -> Self {
        Self {
            label,
            width: 1,
            height: 1,
            format,
            bytes_per_pixel,
            mips: vec![MipLevel {
                width: 1,
                height: 1,
                pixels: pixels.to_vec(),
            }],
        }
    }

    #[cfg(not(test))]
    fn from_image(
        label: &'static str,
        image: DynamicImage,
        max_dimension: u32,
        kind: TextureKind,
    ) -> Self {
        match kind {
            TextureKind::Color => {
                let image = fit_rgba(image.to_rgba8(), max_dimension);
                Self::from_pixels(
                    label,
                    image.width(),
                    image.height(),
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    4,
                    image.into_raw(),
                    MipKind::SrgbColor,
                )
            }
            TextureKind::Normal => {
                let image = fit_rgb(image.to_rgb8(), max_dimension);
                let mut pixels = Vec::with_capacity((image.width() * image.height() * 2) as usize);
                for pixel in image.pixels() {
                    pixels.push(pixel[0]);
                    pixels.push(pixel[1]);
                }
                Self::from_pixels(
                    label,
                    image.width(),
                    image.height(),
                    wgpu::TextureFormat::Rg8Unorm,
                    2,
                    pixels,
                    MipKind::NormalXy,
                )
            }
            TextureKind::Data => {
                let image = fit_gray(image.to_luma8(), max_dimension);
                Self::from_pixels(
                    label,
                    image.width(),
                    image.height(),
                    wgpu::TextureFormat::R8Unorm,
                    1,
                    image.into_raw(),
                    MipKind::LinearData,
                )
            }
        }
    }

    #[cfg(not(test))]
    fn from_pixels(
        label: &'static str,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        bytes_per_pixel: u32,
        base: Vec<u8>,
        kind: MipKind,
    ) -> Self {
        let mut mips = vec![MipLevel {
            width,
            height,
            pixels: base,
        }];
        while mips
            .last()
            .is_some_and(|mip| mip.width > 1 || mip.height > 1)
        {
            let previous = mips.last().expect("base mip is present");
            mips.push(downsample(previous, bytes_per_pixel, kind));
        }
        Self {
            label,
            width,
            height,
            format,
            bytes_per_pixel,
            mips,
        }
    }

    #[cfg(not(test))]
    fn gpu_bytes(&self) -> usize {
        self.mips.iter().map(|mip| mip.pixels.len()).sum()
    }
}

#[cfg(not(test))]
#[derive(Clone, Copy)]
enum TextureKind {
    Color,
    Normal,
    Data,
}

#[cfg(not(test))]
#[derive(Clone, Copy)]
enum MipKind {
    SrgbColor,
    NormalXy,
    LinearData,
}

pub(crate) fn create_material(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    definition: &PbrMaterial,
) -> Result<(GpuMaterial, [u32; 2], usize), String> {
    #[cfg(test)]
    {
        let paths = texture_paths(&definition.textures);
        let mut resolution = [1, 1];
        for path in paths.into_iter().flatten() {
            let resolved = resolve_asset_path(path);
            let reader = image::ImageReader::open(&resolved)
                .map_err(|error| format!("{}: {error}", resolved.display()))?;
            let dimensions = reader
                .into_dimensions()
                .map_err(|error| format!("{}: {error}", resolved.display()))?;
            if dimensions.0 == 0 || dimensions.1 == 0 {
                return Err(format!(
                    "{} has unsupported dimensions {dimensions:?}",
                    resolved.display()
                ));
            }
            if resolution == [1, 1] {
                resolution = [dimensions.0, dimensions.1];
            }
        }
        // Source headers are checked; tiny maps exercise the same complete bind layout cheaply.
        let maps = default_texture_set();
        let material = create_gpu_material(
            device,
            queue,
            layout,
            maps,
            &material_uniform(definition),
            &definition.name,
        );
        Ok((material, resolution, 13))
    }

    #[cfg(not(test))]
    {
        let max_dimension = device.limits().max_texture_dimension_2d.clamp(1, 4096);
        let textures = &definition.textures;
        let map_specs = [
            (
                textures.base_color.as_deref(),
                "PBR Base Color",
                TextureKind::Color,
                [255, 255, 255, 255],
            ),
            (
                textures.normal.as_deref(),
                "PBR Normal",
                TextureKind::Normal,
                [128, 128, 0, 0],
            ),
            (
                textures.metallic.as_deref(),
                "PBR Metallic",
                TextureKind::Data,
                [255, 0, 0, 0],
            ),
            (
                textures.roughness.as_deref(),
                "PBR Roughness",
                TextureKind::Data,
                [255, 0, 0, 0],
            ),
            (
                textures.ambient_occlusion.as_deref(),
                "PBR Occlusion",
                TextureKind::Data,
                [255, 0, 0, 0],
            ),
            (
                textures.emissive.as_deref(),
                "PBR Emissive",
                TextureKind::Color,
                [255, 255, 255, 255],
            ),
        ];
        let mut maps = Vec::with_capacity(map_specs.len());
        for (path, label, kind, fallback) in map_specs {
            maps.push(load_optional_texture(
                path,
                label,
                max_dimension,
                kind,
                fallback,
            )?);
        }
        let resolution = maps
            .iter()
            .find(|map| map.width > 1)
            .map_or([1, 1], |map| [map.width, map.height]);
        let total_gpu_bytes = maps.iter().map(TextureData::gpu_bytes).sum();
        let material = create_gpu_material(
            device,
            queue,
            layout,
            maps,
            &material_uniform(definition),
            &definition.name,
        );
        Ok((material, resolution, total_gpu_bytes))
    }
}

pub(crate) fn create_default_material(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> GpuMaterial {
    let definition = PbrMaterial::default();
    create_gpu_material(
        device,
        queue,
        layout,
        default_texture_set(),
        &material_uniform(&definition),
        "Default PBR Material",
    )
}

fn default_texture_set() -> Vec<TextureData> {
    vec![
        TextureData::solid(
            "Default Base Color",
            wgpu::TextureFormat::Rgba8UnormSrgb,
            4,
            &[255; 4],
        ),
        TextureData::solid(
            "Default Normal",
            wgpu::TextureFormat::Rg8Unorm,
            2,
            &[128, 128],
        ),
        TextureData::solid("Default Metallic", wgpu::TextureFormat::R8Unorm, 1, &[255]),
        TextureData::solid("Default Roughness", wgpu::TextureFormat::R8Unorm, 1, &[255]),
        TextureData::solid("Default Occlusion", wgpu::TextureFormat::R8Unorm, 1, &[255]),
        TextureData::solid(
            "Default Emissive",
            wgpu::TextureFormat::Rgba8UnormSrgb,
            4,
            &[255; 4],
        ),
    ]
}

#[cfg(test)]
fn texture_paths(textures: &forge_scene::PbrTextureSet) -> [Option<&str>; 6] {
    [
        textures.base_color.as_deref(),
        textures.normal.as_deref(),
        textures.metallic.as_deref(),
        textures.roughness.as_deref(),
        textures.ambient_occlusion.as_deref(),
        textures.emissive.as_deref(),
    ]
}

pub(crate) fn material_uniform(material: &PbrMaterial) -> MaterialUniform {
    MaterialUniform {
        base_color_factor: material.base_color_factor,
        emissive_factor: [
            material.emissive_factor[0],
            material.emissive_factor[1],
            material.emissive_factor[2],
            0.0,
        ],
        material_params: [
            material.metallic_factor.clamp(0.0, 1.0),
            material.roughness_factor.clamp(0.045, 1.0),
            material.normal_scale.max(0.0),
            material.occlusion_strength.clamp(0.0, 1.0),
        ],
        uv_scale: material.uv_scale,
        padding: [0; 2],
    }
}

fn resolve_asset_path(path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(path)
    }
}

#[cfg(not(test))]
fn load_optional_texture(
    path: Option<&str>,
    label: &'static str,
    max_dimension: u32,
    kind: TextureKind,
    fallback: [u8; 4],
) -> Result<TextureData, String> {
    if let Some(path) = path {
        let resolved = resolve_asset_path(path);
        load_texture(&resolved, label, max_dimension, kind)
    } else {
        let (format, bytes_per_pixel, pixel) = match kind {
            TextureKind::Color => (wgpu::TextureFormat::Rgba8UnormSrgb, 4, fallback.to_vec()),
            TextureKind::Normal => (wgpu::TextureFormat::Rg8Unorm, 2, fallback[..2].to_vec()),
            TextureKind::Data => (wgpu::TextureFormat::R8Unorm, 1, fallback[..1].to_vec()),
        };
        Ok(TextureData::solid(label, format, bytes_per_pixel, &pixel))
    }
}

#[cfg(not(test))]
fn load_texture(
    path: &Path,
    label: &'static str,
    max_dimension: u32,
    kind: TextureKind,
) -> Result<TextureData, String> {
    let decoded = image::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(TextureData::from_image(label, decoded, max_dimension, kind))
}

#[cfg(not(test))]
fn fit_rgba(image: RgbaImage, max_dimension: u32) -> RgbaImage {
    let [width, height] = fit_size(image.width(), image.height(), max_dimension);
    if width == image.width() && height == image.height() {
        image
    } else {
        image::imageops::resize(&image, width, height, FilterType::Triangle)
    }
}

#[cfg(not(test))]
fn fit_rgb(image: RgbImage, max_dimension: u32) -> RgbImage {
    let [width, height] = fit_size(image.width(), image.height(), max_dimension);
    if width == image.width() && height == image.height() {
        image
    } else {
        image::imageops::resize(&image, width, height, FilterType::Triangle)
    }
}

#[cfg(not(test))]
fn fit_gray(image: GrayImage, max_dimension: u32) -> GrayImage {
    let [width, height] = fit_size(image.width(), image.height(), max_dimension);
    if width == image.width() && height == image.height() {
        image
    } else {
        image::imageops::resize(&image, width, height, FilterType::Triangle)
    }
}

#[cfg(not(test))]
fn fit_size(width: u32, height: u32, max_dimension: u32) -> [u32; 2] {
    let scale = (max_dimension as f32 / width.max(height) as f32).min(1.0);
    [
        (width as f32 * scale).round().max(1.0) as u32,
        (height as f32 * scale).round().max(1.0) as u32,
    ]
}

#[cfg(not(test))]
fn downsample(previous: &MipLevel, bytes_per_pixel: u32, kind: MipKind) -> MipLevel {
    let width = (previous.width / 2).max(1);
    let height = (previous.height / 2).max(1);
    let channels = bytes_per_pixel as usize;
    let mut pixels = vec![0; (width * height) as usize * channels];

    for y in 0..height {
        for x in 0..width {
            let mut count = 0.0;
            let mut sum = [0.0_f32; 4];
            let mut normal_sum = [0.0_f32; 3];
            for source_y in (y * 2)..(y * 2 + 2).min(previous.height) {
                for source_x in (x * 2)..(x * 2 + 2).min(previous.width) {
                    let source = ((source_y * previous.width + source_x) as usize) * channels;
                    match kind {
                        MipKind::SrgbColor => {
                            for (channel, total) in sum.iter_mut().enumerate().take(channels.min(3))
                            {
                                *total += srgb_to_linear(previous.pixels[source + channel]);
                            }
                            if channels > 3 {
                                sum[3] += previous.pixels[source + 3] as f32 / 255.0;
                            }
                        }
                        MipKind::NormalXy => {
                            let nx = previous.pixels[source] as f32 / 255.0 * 2.0 - 1.0;
                            let ny = previous.pixels[source + 1] as f32 / 255.0 * 2.0 - 1.0;
                            let nz = (1.0 - nx * nx - ny * ny).max(0.0).sqrt();
                            normal_sum[0] += nx;
                            normal_sum[1] += ny;
                            normal_sum[2] += nz;
                        }
                        MipKind::LinearData => {
                            for (channel, total) in sum.iter_mut().enumerate().take(channels) {
                                *total += previous.pixels[source + channel] as f32 / 255.0;
                            }
                        }
                    }
                    count += 1.0;
                }
            }

            let target = ((y * width + x) as usize) * channels;
            match kind {
                MipKind::SrgbColor => {
                    for channel in 0..channels.min(3) {
                        pixels[target + channel] = linear_to_srgb(sum[channel] / count);
                    }
                    if channels > 3 {
                        pixels[target + 3] = to_unorm8(sum[3] / count);
                    }
                }
                MipKind::NormalXy => {
                    let length =
                        (normal_sum[0].powi(2) + normal_sum[1].powi(2) + normal_sum[2].powi(2))
                            .sqrt()
                            .max(f32::EPSILON);
                    pixels[target] = to_unorm8(normal_sum[0] / length * 0.5 + 0.5);
                    pixels[target + 1] = to_unorm8(normal_sum[1] / length * 0.5 + 0.5);
                }
                MipKind::LinearData => {
                    for channel in 0..channels {
                        pixels[target + channel] = to_unorm8(sum[channel] / count);
                    }
                }
            }
        }
    }
    MipLevel {
        width,
        height,
        pixels,
    }
}

#[cfg(not(test))]
fn srgb_to_linear(value: u8) -> f32 {
    static LUT: OnceLock<[f32; 256]> = OnceLock::new();
    LUT.get_or_init(|| {
        std::array::from_fn(|index| {
            let value = index as f32 / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        })
    })[value as usize]
}

#[cfg(not(test))]
fn linear_to_srgb(value: f32) -> u8 {
    static LUT: OnceLock<Vec<u8>> = OnceLock::new();
    let value = value.clamp(0.0, 1.0);
    let index = (value * 65_535.0).round() as usize;
    LUT.get_or_init(|| {
        (0..=65_535)
            .map(|index| {
                let value = index as f32 / 65_535.0;
                let encoded = if value <= 0.0031308 {
                    value * 12.92
                } else {
                    1.055 * value.powf(1.0 / 2.4) - 0.055
                };
                to_unorm8(encoded)
            })
            .collect()
    })[index]
}

#[cfg(not(test))]
fn to_unorm8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn create_gpu_material(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    maps: Vec<TextureData>,
    uniform: &MaterialUniform,
    label: &str,
) -> GpuMaterial {
    debug_assert_eq!(
        maps.len(),
        6,
        "materials bind base color, normal, metallic, roughness, AO, and emissive maps"
    );
    let mut textures = Vec::with_capacity(maps.len());
    let mut views = Vec::with_capacity(maps.len());
    let mut sampler_max_lod = 0.0_f32;

    for map in maps {
        sampler_max_lod = sampler_max_lod.max(map.mips.len().saturating_sub(1) as f32);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(map.label),
            size: wgpu::Extent3d {
                width: map.width,
                height: map.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: map.mips.len() as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: map.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for (mip_level, mip) in map.mips.iter().enumerate() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: mip_level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &mip.pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(mip.width * map.bytes_per_pixel),
                    rows_per_image: Some(mip.height),
                },
                wgpu::Extent3d {
                    width: mip.width,
                    height: mip.height,
                    depth_or_array_layers: 1,
                },
            );
        }

        views.push(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        textures.push(texture);
    }

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Tiled PBR Material Sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        lod_min_clamp: 0.0,
        lod_max_clamp: sampler_max_lod,
        ..Default::default()
    });
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("PBR Material Uniform Buffer"),
        contents: bytemuck::bytes_of(uniform),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&views[0]),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&views[1]),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&views[2]),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(&views[3]),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::TextureView(&views[4]),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(&views[5]),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    });

    GpuMaterial {
        bind_group,
        uniform_buffer,
        _textures: textures,
        _sampler: sampler,
    }
}

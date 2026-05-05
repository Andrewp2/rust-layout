use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

#[cfg(not(target_arch = "wasm32"))]
use std::{fs, io};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaderBackend {
    Spirv,
    Wgsl,
}

pub const LAYOUT_FILL_SHADER: &str = "layout_fill";
pub const LAYOUT_PICK_SHADER: &str = "layout_pick";

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("renderer crate lives under crates/renderer")
        .to_path_buf()
}

pub fn compiled_shader_path(id: &str, backend: ShaderBackend) -> PathBuf {
    let root = workspace_root().join("assets/shaders/compiled_shaders");
    match backend {
        ShaderBackend::Spirv => root.join("spirv").join(format!("{id}.spv")),
        ShaderBackend::Wgsl => root.join("wgsl").join(format!("{id}.wgsl")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn preferred_backend() -> ShaderBackend {
    ShaderBackend::Spirv
}

#[cfg(target_arch = "wasm32")]
pub fn preferred_backend() -> ShaderBackend {
    ShaderBackend::Wgsl
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_shader_module(
    device: &wgpu::Device,
    id: &str,
    backend: ShaderBackend,
) -> Result<wgpu::ShaderModule, io::Error> {
    match backend {
        ShaderBackend::Spirv => load_spirv_from_file(device, compiled_shader_path(id, backend)),
        ShaderBackend::Wgsl => load_wgsl_from_file(device, compiled_shader_path(id, backend)),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_spirv_from_file(
    device: &wgpu::Device,
    path: impl AsRef<Path>,
) -> Result<wgpu::ShaderModule, io::Error> {
    let path = path.as_ref();
    let bytes = fs::read(path)?;
    if bytes.len() < 4 || bytes.len() % 4 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("SPIR-V file {} is not 4-byte aligned", path.display()),
        ));
    }
    Ok(unsafe {
        device.create_shader_module_passthrough(wgpu::ShaderModuleDescriptorPassthrough {
            label: path.file_name().and_then(|name| name.to_str()),
            spirv: Some(wgpu::util::make_spirv_raw(&bytes)),
            ..Default::default()
        })
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_wgsl_from_file(
    device: &wgpu::Device,
    path: impl AsRef<Path>,
) -> Result<wgpu::ShaderModule, io::Error> {
    let path = path.as_ref();
    let source = fs::read_to_string(path)?;
    Ok(device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: path.file_name().and_then(|name| name.to_str()),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(source)),
    }))
}

#[cfg(target_arch = "wasm32")]
pub fn load_embedded_layout_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    const WGSL: &str =
        include_str!("../../../assets/shaders/compiled_shaders/wgsl/layout_fill.wgsl");
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("layout_fill.wgsl"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(WGSL)),
    })
}

#[cfg(target_arch = "wasm32")]
pub fn load_embedded_pick_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    const WGSL: &str =
        include_str!("../../../assets/shaders/compiled_shaders/wgsl/layout_pick.wgsl");
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("layout_pick.wgsl"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(WGSL)),
    })
}

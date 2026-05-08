use fabricad_app::{Benchmark3dOptions, OffscreenRenderOptions, OffscreenScene, StartupOptions};
use std::{path::PathBuf, sync::Arc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    let launch = LaunchOptions::from_env_and_args(std::env::args().skip(1))?;
    if launch.help {
        print_usage();
        return Ok(());
    }
    if let Some(path) = launch.export_gds {
        fabricad_app::export_demo_gds(&path)?;
        println!("exported GDSII {}", path.display());
        return Ok(());
    }
    if let Some(options) = launch.offscreen {
        let report = fabricad_app::run_offscreen_render(options)?;
        println!("{}", report.summary());
        return Ok(());
    }
    let startup_options = launch.startup_options();

    let wgpu_options = wgpu_configuration(launch.benchmark_3d.is_some());
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Fabricad")
            .with_inner_size([1440.0, 920.0]),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options,
        ..Default::default()
    };
    eframe::run_native(
        "Fabricad",
        options,
        Box::new(|cc| {
            Ok(Box::new(fabricad_app::FabricadApp::new_with_options(
                cc,
                startup_options,
            )))
        }),
    )?;
    Ok(())
}

fn wgpu_configuration(benchmark_3d: bool) -> egui_wgpu::WgpuConfiguration {
    let mut config = if benchmark_3d {
        egui_wgpu::WgpuConfiguration {
            present_mode: egui_wgpu::wgpu::PresentMode::AutoNoVsync,
            desired_maximum_frame_latency: Some(1),
            ..Default::default()
        }
    } else {
        Default::default()
    };
    if let egui_wgpu::WgpuSetup::CreateNew(create_new) = &mut config.wgpu_setup {
        create_new.native_adapter_selector = Some(Arc::new(move |adapters, surface| {
            select_preferred_native_adapter(adapters, surface, benchmark_3d)
        }));
    }
    config
}

fn select_preferred_native_adapter(
    adapters: &[egui_wgpu::wgpu::Adapter],
    surface: Option<&egui_wgpu::wgpu::Surface<'_>>,
    require_hardware: bool,
) -> Result<egui_wgpu::wgpu::Adapter, String> {
    let adapter = adapters
        .iter()
        .filter(|adapter| surface.is_none_or(|surface| adapter.is_surface_supported(surface)))
        .min_by_key(|adapter| {
            let info = adapter.get_info();
            (
                native_adapter_device_type_rank(info.device_type),
                native_adapter_backend_rank(info.backend),
                info.name,
            )
        })
        .cloned()
        .ok_or_else(|| "no compatible wgpu adapters found".to_string())?;

    let info = adapter.get_info();
    if require_hardware && !native_adapter_is_hardware(info.device_type) {
        return Err(format!(
            "3D benchmark requires a hardware wgpu adapter, but selected {}; available adapters: {}",
            native_adapter_summary(&info),
            native_adapter_list_summary(adapters)
        ));
    }

    Ok(adapter)
}

fn native_adapter_is_hardware(device_type: egui_wgpu::wgpu::DeviceType) -> bool {
    matches!(
        device_type,
        egui_wgpu::wgpu::DeviceType::DiscreteGpu | egui_wgpu::wgpu::DeviceType::IntegratedGpu
    )
}

fn native_adapter_device_type_rank(device_type: egui_wgpu::wgpu::DeviceType) -> u8 {
    match device_type {
        egui_wgpu::wgpu::DeviceType::DiscreteGpu => 0,
        egui_wgpu::wgpu::DeviceType::IntegratedGpu => 1,
        egui_wgpu::wgpu::DeviceType::VirtualGpu => 2,
        egui_wgpu::wgpu::DeviceType::Other => 3,
        egui_wgpu::wgpu::DeviceType::Cpu => 4,
    }
}

fn native_adapter_backend_rank(backend: egui_wgpu::wgpu::Backend) -> u8 {
    match backend {
        egui_wgpu::wgpu::Backend::Vulkan
        | egui_wgpu::wgpu::Backend::Metal
        | egui_wgpu::wgpu::Backend::Dx12 => 0,
        egui_wgpu::wgpu::Backend::Gl => 1,
        egui_wgpu::wgpu::Backend::BrowserWebGpu => 2,
        egui_wgpu::wgpu::Backend::Noop => 3,
    }
}

fn native_adapter_list_summary(adapters: &[egui_wgpu::wgpu::Adapter]) -> String {
    if adapters.is_empty() {
        return "none".to_string();
    }
    adapters
        .iter()
        .map(|adapter| native_adapter_summary(&adapter.get_info()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn native_adapter_summary(info: &egui_wgpu::wgpu::AdapterInfo) -> String {
    format!(
        "{:?} {:?} \"{}\"",
        info.backend, info.device_type, info.name
    )
}

fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|err| {
        eprintln!("WARN invalid RUST_LOG filter; using warn: {err}");
        tracing_subscriber::EnvFilter::new("warn")
    });
    if let Err(err) = tracing_subscriber::fmt().with_env_filter(filter).try_init() {
        eprintln!("WARN logging initialization skipped: {err}");
    }
}

#[derive(Debug, PartialEq)]
struct LaunchOptions {
    help: bool,
    offscreen: Option<OffscreenRenderOptions>,
    export_gds: Option<PathBuf>,
    benchmark_3d: Option<Benchmark3dLaunch>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Benchmark3dLaunch {
    count: usize,
    options: Benchmark3dOptions,
}

impl LaunchOptions {
    fn from_env_and_args(
        args: impl IntoIterator<Item = String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::parse(args, env_flag("FABRICAD_OFFSCREEN")).map_err(|message| {
            Box::new(std::io::Error::other(message)) as Box<dyn std::error::Error>
        })
    }

    fn parse(args: impl IntoIterator<Item = String>, env_offscreen: bool) -> Result<Self, String> {
        let mut help = false;
        let mut offscreen_requested = env_offscreen;
        let mut offscreen = OffscreenRenderOptions::default();
        let mut export_gds = None;
        let mut benchmark_3d = None;
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => {
                    help = true;
                }
                "--offscreen" | "--headless" => {
                    offscreen_requested = true;
                }
                "--scene" => {
                    let value = next_value(&mut args, "--scene")?;
                    offscreen.scene = parse_scene(&value)?;
                    offscreen_requested = true;
                }
                "--count" => {
                    let value = next_value(&mut args, "--count")?;
                    offscreen.scene = OffscreenScene::Stress {
                        count: parse_usize(&value, "--count")?,
                    };
                    offscreen_requested = true;
                }
                "--width" => {
                    let value = next_value(&mut args, "--width")?;
                    offscreen.width = parse_u32(&value, "--width")?;
                    offscreen_requested = true;
                }
                "--height" => {
                    let value = next_value(&mut args, "--height")?;
                    offscreen.height = parse_u32(&value, "--height")?;
                    offscreen_requested = true;
                }
                "--zoom" => {
                    let value = next_value(&mut args, "--zoom")?;
                    offscreen.zoom = parse_f32(&value, "--zoom")?;
                    offscreen_requested = true;
                }
                "--pan" => {
                    let value = next_value(&mut args, "--pan")?;
                    offscreen.pan = parse_pan(&value)?;
                    offscreen_requested = true;
                }
                "--export-gds" => {
                    export_gds = Some(PathBuf::from(next_value(&mut args, "--export-gds")?));
                }
                "--bench-3d" => {
                    benchmark_3d.get_or_insert_with(Benchmark3dLaunch::default);
                }
                "--bench-3d-count" => {
                    let value = next_value(&mut args, "--bench-3d-count")?;
                    benchmark_3d
                        .get_or_insert_with(Benchmark3dLaunch::default)
                        .count = parse_usize(&value, "--bench-3d-count")?;
                }
                "--bench-3d-frames" => {
                    let value = next_value(&mut args, "--bench-3d-frames")?;
                    benchmark_3d
                        .get_or_insert_with(Benchmark3dLaunch::default)
                        .options
                        .frames = parse_usize(&value, "--bench-3d-frames")?;
                }
                "--bench-3d-warmup" => {
                    let value = next_value(&mut args, "--bench-3d-warmup")?;
                    benchmark_3d
                        .get_or_insert_with(Benchmark3dLaunch::default)
                        .options
                        .warmup_frames = parse_usize(&value, "--bench-3d-warmup")?;
                }
                _ => {
                    if let Some(value) = arg.strip_prefix("--scene=") {
                        offscreen.scene = parse_scene(value)?;
                        offscreen_requested = true;
                    } else if let Some(value) = arg.strip_prefix("--count=") {
                        offscreen.scene = OffscreenScene::Stress {
                            count: parse_usize(value, "--count")?,
                        };
                        offscreen_requested = true;
                    } else if let Some(value) = arg.strip_prefix("--width=") {
                        offscreen.width = parse_u32(value, "--width")?;
                        offscreen_requested = true;
                    } else if let Some(value) = arg.strip_prefix("--height=") {
                        offscreen.height = parse_u32(value, "--height")?;
                        offscreen_requested = true;
                    } else if let Some(value) = arg.strip_prefix("--zoom=") {
                        offscreen.zoom = parse_f32(value, "--zoom")?;
                        offscreen_requested = true;
                    } else if let Some(value) = arg.strip_prefix("--pan=") {
                        offscreen.pan = parse_pan(value)?;
                        offscreen_requested = true;
                    } else if let Some(value) = arg.strip_prefix("--export-gds=") {
                        export_gds = Some(PathBuf::from(value));
                    } else if let Some(value) = arg.strip_prefix("--bench-3d-count=") {
                        benchmark_3d
                            .get_or_insert_with(Benchmark3dLaunch::default)
                            .count = parse_usize(value, "--bench-3d-count")?;
                    } else if let Some(value) = arg.strip_prefix("--bench-3d-frames=") {
                        benchmark_3d
                            .get_or_insert_with(Benchmark3dLaunch::default)
                            .options
                            .frames = parse_usize(value, "--bench-3d-frames")?;
                    } else if let Some(value) = arg.strip_prefix("--bench-3d-warmup=") {
                        benchmark_3d
                            .get_or_insert_with(Benchmark3dLaunch::default)
                            .options
                            .warmup_frames = parse_usize(value, "--bench-3d-warmup")?;
                    } else {
                        return Err(format!("unknown argument {arg:?}; use --help"));
                    }
                }
            }
        }

        Ok(Self {
            help,
            offscreen: offscreen_requested.then_some(offscreen),
            export_gds,
            benchmark_3d,
        })
    }

    fn startup_options(&self) -> StartupOptions {
        if let Some(benchmark) = self.benchmark_3d {
            StartupOptions {
                stress_count: Some(benchmark.count),
                view_3d: true,
                benchmark_3d: Some(benchmark.options),
                ..Default::default()
            }
        } else {
            StartupOptions::default()
        }
    }
}

impl Default for Benchmark3dLaunch {
    fn default() -> Self {
        Self {
            count: 10_000,
            options: Benchmark3dOptions::default(),
        }
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_scene(value: &str) -> Result<OffscreenScene, String> {
    match value {
        "demo" => Ok(OffscreenScene::Demo),
        "hierarchy" => Ok(OffscreenScene::Hierarchy),
        "stress" => Ok(OffscreenScene::Stress { count: 20_000 }),
        _ => Err(format!(
            "unsupported scene {value:?}; expected demo, hierarchy, or stress"
        )),
    }
}

fn parse_u32(value: &str, flag: &str) -> Result<u32, String> {
    value
        .parse()
        .map_err(|_| format!("{flag} must be an unsigned integer"))
}

fn parse_usize(value: &str, flag: &str) -> Result<usize, String> {
    value
        .parse()
        .map_err(|_| format!("{flag} must be an unsigned integer"))
}

fn parse_f32(value: &str, flag: &str) -> Result<f32, String> {
    let parsed: f32 = value
        .parse()
        .map_err(|_| format!("{flag} must be a number"))?;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(format!("{flag} must be finite"))
    }
}

fn parse_pan(value: &str) -> Result<[f32; 2], String> {
    let (x, y) = value
        .split_once(',')
        .ok_or_else(|| "--pan must be formatted as x,y".to_string())?;
    Ok([parse_f32(x, "--pan x")?, parse_f32(y, "--pan y")?])
}

fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn print_usage() {
    println!(
        "Fabricad\n\n\
         Usage:\n  \
         fabricad\n  \
         fabricad --offscreen [--scene demo|hierarchy|stress] [--count N] [--width W] [--height H] [--zoom Z] [--pan X,Y]\n\n\
         fabricad --bench-3d [--bench-3d-count N] [--bench-3d-frames N] [--bench-3d-warmup N]\n\n\
         fabricad --export-gds PATH\n\n\
         Set FABRICAD_OFFSCREEN=1 to make the binary use offscreen rendering by default."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_offscreen_flag() {
        let launch = LaunchOptions::parse(["--offscreen".to_string()], false).unwrap();
        assert_eq!(launch.offscreen, Some(OffscreenRenderOptions::default()));
        assert_eq!(launch.export_gds, None);
        assert_eq!(launch.benchmark_3d, None);
    }

    #[test]
    fn scene_argument_implies_offscreen() {
        let launch = LaunchOptions::parse(
            [
                "--scene=stress".to_string(),
                "--count".to_string(),
                "42".to_string(),
            ],
            false,
        )
        .unwrap();
        let options = launch.offscreen.unwrap();
        assert_eq!(options.scene, OffscreenScene::Stress { count: 42 });
    }

    #[test]
    fn env_flag_enables_offscreen() {
        let launch = LaunchOptions::parse(Vec::<String>::new(), true).unwrap();
        assert_eq!(launch.offscreen, Some(OffscreenRenderOptions::default()));
    }

    #[test]
    fn parses_gds_export_flag() {
        let launch = LaunchOptions::parse(
            ["--export-gds".to_string(), "target/demo.gds".to_string()],
            false,
        )
        .unwrap();

        assert_eq!(launch.offscreen, None);
        assert_eq!(launch.export_gds, Some(PathBuf::from("target/demo.gds")));
        assert_eq!(launch.benchmark_3d, None);
    }

    #[test]
    fn parses_3d_benchmark_options() {
        let launch = LaunchOptions::parse(
            [
                "--bench-3d".to_string(),
                "--bench-3d-count=10000".to_string(),
                "--bench-3d-frames".to_string(),
                "120".to_string(),
                "--bench-3d-warmup".to_string(),
                "12".to_string(),
            ],
            false,
        )
        .unwrap();

        assert_eq!(launch.offscreen, None);
        assert_eq!(
            launch.benchmark_3d,
            Some(Benchmark3dLaunch {
                count: 10_000,
                options: Benchmark3dOptions {
                    frames: 120,
                    warmup_frames: 12,
                },
            })
        );
        assert_eq!(
            launch.startup_options().benchmark_3d,
            Some(Benchmark3dOptions {
                frames: 120,
                warmup_frames: 12,
            })
        );
    }

    #[test]
    fn native_adapter_ranking_prefers_hardware_gpus() {
        use egui_wgpu::wgpu::DeviceType;

        assert!(native_adapter_is_hardware(DeviceType::DiscreteGpu));
        assert!(native_adapter_is_hardware(DeviceType::IntegratedGpu));
        assert!(!native_adapter_is_hardware(DeviceType::Cpu));
        assert!(
            native_adapter_device_type_rank(DeviceType::DiscreteGpu)
                < native_adapter_device_type_rank(DeviceType::IntegratedGpu)
        );
        assert!(
            native_adapter_device_type_rank(DeviceType::IntegratedGpu)
                < native_adapter_device_type_rank(DeviceType::Cpu)
        );
    }

    #[test]
    fn native_adapter_ranking_prefers_primary_graphics_backends() {
        use egui_wgpu::wgpu::Backend;

        assert!(
            native_adapter_backend_rank(Backend::Vulkan) < native_adapter_backend_rank(Backend::Gl)
        );
        assert!(
            native_adapter_backend_rank(Backend::Metal) < native_adapter_backend_rank(Backend::Gl)
        );
        assert!(
            native_adapter_backend_rank(Backend::Dx12) < native_adapter_backend_rank(Backend::Gl)
        );
    }
}

use fabricad_app::{
    Benchmark3dOptions, OffscreenRenderOptions, OffscreenScene, OperadSnapshotReport,
    StartupOptions, StartupView, render_operad_snapshot, run_operad_audit,
};
use operad::ResourceFormat;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

mod native_window;

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
    if let Some((width, height)) = launch.operad_snapshot {
        let report = render_operad_snapshot(startup_options, width, height)?;
        if let Some(path) = &launch.snapshot_rgba {
            write_snapshot_rgba(path, &report)?;
        }
        println!("{}", report.summary());
    } else if launch.audit {
        let report = run_operad_audit(startup_options)?;
        println!("{}", report.summary());
    } else {
        native_window::run(startup_options)?;
    }
    Ok(())
}

fn init_logging() {
    let filter = match std::env::var("RUST_LOG") {
        Ok(value) => tracing_subscriber::EnvFilter::try_new(value).unwrap_or_else(|err| {
            eprintln!("WARN invalid RUST_LOG filter; using Fabricad defaults: {err}");
            default_log_filter()
        }),
        Err(_) => default_log_filter(),
    };
    if let Err(err) = tracing_subscriber::fmt().with_env_filter(filter).try_init() {
        eprintln!("WARN logging initialization skipped: {err}");
    }
}

fn default_log_filter() -> tracing_subscriber::EnvFilter {
    tracing_subscriber::EnvFilter::new("fabricad_app=warn,renderer=warn,sync_server=warn")
}

#[derive(Debug, PartialEq)]
struct LaunchOptions {
    help: bool,
    audit: bool,
    offscreen: Option<OffscreenRenderOptions>,
    export_gds: Option<PathBuf>,
    benchmark_3d: Option<Benchmark3dLaunch>,
    operad_snapshot: Option<(u32, u32)>,
    snapshot_rgba: Option<PathBuf>,
    startup_options: StartupOptions,
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
        let mut audit = false;
        let mut offscreen_requested = env_offscreen;
        let mut offscreen = OffscreenRenderOptions::default();
        let mut export_gds = None;
        let mut benchmark_3d = None;
        let mut operad_snapshot = None;
        let mut snapshot_rgba = None;
        let mut startup_options = StartupOptions::default();
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => help = true,
                "--audit" | "--operad-audit" => audit = true,
                "--offscreen" | "--headless" => offscreen_requested = true,
                "--operad-snapshot" => operad_snapshot = Some((1440, 920)),
                "--snapshot-rgba" => {
                    snapshot_rgba = Some(PathBuf::from(next_value(&mut args, "--snapshot-rgba")?));
                    operad_snapshot.get_or_insert((1440, 920));
                }
                "--workspace" => {
                    apply_workspace(&mut startup_options, &next_value(&mut args, "--workspace")?)?;
                }
                "--view" => {
                    apply_view(&mut startup_options, &next_value(&mut args, "--view")?)?;
                }
                "--options" => startup_options.show_options = true,
                "--select" => {
                    apply_select(&mut startup_options, &next_value(&mut args, "--select")?)?;
                }
                "--edit" => {
                    apply_edit(&mut startup_options, &next_value(&mut args, "--edit")?)?;
                }
                "--scene" => {
                    let value = next_value(&mut args, "--scene")?;
                    offscreen.scene = parse_scene(&value)?;
                    let stress_count = startup_options.stress_count;
                    apply_scene(&mut startup_options, &value, stress_count)?;
                    offscreen_requested = true;
                }
                "--count" => {
                    let value = next_value(&mut args, "--count")?;
                    let count = parse_usize(&value, "--count")?;
                    offscreen.scene = OffscreenScene::Stress { count };
                    startup_options.stress_count = Some(count);
                    offscreen_requested = true;
                }
                "--width" => {
                    let value = next_value(&mut args, "--width")?;
                    offscreen.width = parse_u32(&value, "--width")?;
                    if let Some((_, height)) = operad_snapshot {
                        operad_snapshot = Some((offscreen.width, height));
                    }
                    offscreen_requested = true;
                }
                "--height" => {
                    let value = next_value(&mut args, "--height")?;
                    offscreen.height = parse_u32(&value, "--height")?;
                    if let Some((width, _)) = operad_snapshot {
                        operad_snapshot = Some((width, offscreen.height));
                    }
                    offscreen_requested = true;
                }
                "--zoom" => {
                    let value = next_value(&mut args, "--zoom")?;
                    offscreen.zoom = parse_f32(&value, "--zoom")?;
                    startup_options.zoom = Some(offscreen.zoom);
                    offscreen_requested = true;
                }
                "--pan" => {
                    let value = next_value(&mut args, "--pan")?;
                    offscreen.pan = parse_pan(&value)?;
                    startup_options.pan = Some(offscreen.pan);
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
                        let stress_count = startup_options.stress_count;
                        apply_scene(&mut startup_options, value, stress_count)?;
                        offscreen_requested = true;
                    } else if let Some(value) = arg.strip_prefix("--count=") {
                        let count = parse_usize(value, "--count")?;
                        offscreen.scene = OffscreenScene::Stress { count };
                        startup_options.stress_count = Some(count);
                        offscreen_requested = true;
                    } else if let Some(value) = arg.strip_prefix("--width=") {
                        offscreen.width = parse_u32(value, "--width")?;
                        if let Some((_, height)) = operad_snapshot {
                            operad_snapshot = Some((offscreen.width, height));
                        }
                        offscreen_requested = true;
                    } else if let Some(value) = arg.strip_prefix("--height=") {
                        offscreen.height = parse_u32(value, "--height")?;
                        if let Some((width, _)) = operad_snapshot {
                            operad_snapshot = Some((width, offscreen.height));
                        }
                        offscreen_requested = true;
                    } else if let Some(value) = arg.strip_prefix("--zoom=") {
                        offscreen.zoom = parse_f32(value, "--zoom")?;
                        startup_options.zoom = Some(offscreen.zoom);
                        offscreen_requested = true;
                    } else if let Some(value) = arg.strip_prefix("--pan=") {
                        offscreen.pan = parse_pan(value)?;
                        startup_options.pan = Some(offscreen.pan);
                        offscreen_requested = true;
                    } else if let Some(value) = arg.strip_prefix("--export-gds=") {
                        export_gds = Some(PathBuf::from(value));
                    } else if let Some(value) = arg.strip_prefix("--snapshot-rgba=") {
                        snapshot_rgba = Some(PathBuf::from(value));
                        operad_snapshot.get_or_insert((1440, 920));
                    } else if let Some(value) = arg.strip_prefix("--workspace=") {
                        apply_workspace(&mut startup_options, value)?;
                    } else if let Some(value) = arg.strip_prefix("--view=") {
                        apply_view(&mut startup_options, value)?;
                    } else if let Some(value) = arg.strip_prefix("--select=") {
                        apply_select(&mut startup_options, value)?;
                    } else if let Some(value) = arg.strip_prefix("--edit=") {
                        apply_edit(&mut startup_options, value)?;
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
            audit,
            offscreen: (offscreen_requested && operad_snapshot.is_none()).then_some(offscreen),
            export_gds,
            benchmark_3d,
            operad_snapshot,
            snapshot_rgba,
            startup_options,
        })
    }

    fn startup_options(&self) -> StartupOptions {
        if let Some(benchmark) = self.benchmark_3d {
            StartupOptions {
                stress_count: Some(benchmark.count),
                view_3d: true,
                benchmark_3d: Some(benchmark.options),
                ..self.startup_options
            }
        } else {
            self.startup_options
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

fn apply_workspace(options: &mut StartupOptions, value: &str) -> Result<(), String> {
    match value {
        "demo" => {
            options.demo_workspace = true;
            Ok(())
        }
        _ => Err(format!("unsupported workspace {value:?}; expected demo")),
    }
}

fn apply_scene(
    options: &mut StartupOptions,
    value: &str,
    count: Option<usize>,
) -> Result<(), String> {
    match value {
        "demo" => Ok(()),
        "hierarchy" => {
            options.hierarchy_demo = true;
            Ok(())
        }
        "stress" => {
            options.stress_count = Some(count.unwrap_or(20_000));
            Ok(())
        }
        _ => Err(format!(
            "unsupported scene {value:?}; expected demo, hierarchy, or stress"
        )),
    }
}

fn apply_view(options: &mut StartupOptions, value: &str) -> Result<(), String> {
    let view = StartupView::from_slug(value)
        .ok_or_else(|| format!("unsupported view {value:?}; use a Fabricad view slug"))?;
    options.view_3d = view == StartupView::Layout3d;
    options.view_mode = Some(view);
    Ok(())
}

fn apply_select(options: &mut StartupOptions, value: &str) -> Result<(), String> {
    match value {
        "first" => {
            options.select_first_shape = true;
            Ok(())
        }
        _ => Err(format!("unsupported selection {value:?}; expected first")),
    }
}

fn apply_edit(options: &mut StartupOptions, value: &str) -> Result<(), String> {
    match value {
        "vertex_moved" | "move-first-vertex" => {
            options.move_first_vertex = true;
            Ok(())
        }
        _ => Err(format!(
            "unsupported edit {value:?}; expected vertex_moved or move-first-vertex"
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

fn write_snapshot_rgba(
    path: &Path,
    report: &OperadSnapshotReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let image = report.render.snapshot.as_ref().ok_or_else(|| {
        std::io::Error::other("Operad snapshot render did not produce image pixels")
    })?;
    if image.format != ResourceFormat::Rgba8 {
        return Err(Box::new(std::io::Error::other(format!(
            "unsupported snapshot format {:?}; expected Rgba8",
            image.format
        ))));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::File::create(path)?;
    file.write_all(&image.pixels)?;
    Ok(())
}

fn print_usage() {
    println!(
        "Fabricad\n\n\
         Usage:\n  \
         fabricad\n  \
         fabricad --audit\n  \
         fabricad --operad-snapshot [--width W] [--height H] [--snapshot-rgba PATH] [--view SLUG] [--scene demo|hierarchy|stress]\n  \
         fabricad --offscreen [--scene demo|hierarchy|stress] [--count N] [--width W] [--height H] [--zoom Z] [--pan X,Y]\n  \
         fabricad --bench-3d [--bench-3d-count N] [--bench-3d-frames N] [--bench-3d-warmup N]\n  \
         fabricad --export-gds PATH\n\n\
         The default path opens a native Operad v4 window. Use --audit for the noninteractive summary."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_offscreen_flag() {
        let launch = LaunchOptions::parse(["--offscreen".to_string()], false).unwrap();
        assert!(!launch.audit);
        assert_eq!(launch.offscreen, Some(OffscreenRenderOptions::default()));
        assert_eq!(launch.export_gds, None);
        assert_eq!(launch.benchmark_3d, None);
        assert_eq!(launch.snapshot_rgba, None);
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
    fn parses_operad_snapshot_flag() {
        let launch = LaunchOptions::parse(["--operad-snapshot".to_string()], false).unwrap();
        assert!(!launch.audit);
        assert_eq!(launch.offscreen, None);
        assert_eq!(launch.operad_snapshot, Some((1440, 920)));
        assert_eq!(launch.snapshot_rgba, None);
    }

    #[test]
    fn parses_operad_snapshot_scene_view_and_output() {
        let launch = LaunchOptions::parse(
            [
                "--operad-snapshot".to_string(),
                "--scene=hierarchy".to_string(),
                "--view".to_string(),
                "layout".to_string(),
                "--snapshot-rgba=target/operad.rgba".to_string(),
            ],
            false,
        )
        .unwrap();

        assert_eq!(launch.offscreen, None);
        assert_eq!(launch.operad_snapshot, Some((1440, 920)));
        assert_eq!(
            launch.snapshot_rgba,
            Some(PathBuf::from("target/operad.rgba"))
        );
        assert!(launch.startup_options().hierarchy_demo);
        assert_eq!(
            launch.startup_options().view_mode,
            Some(StartupView::Layout2d)
        );
    }

    #[test]
    fn parses_audit_flag() {
        let launch = LaunchOptions::parse(["--audit".to_string()], false).unwrap();
        assert!(launch.audit);
        assert_eq!(launch.offscreen, None);
        assert_eq!(launch.operad_snapshot, None);
    }
}

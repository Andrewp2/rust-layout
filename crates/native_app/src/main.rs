use fabricad_app::{
    AppOptions, Benchmark3dOptions, OffscreenRenderOptions, OffscreenScene, OperadSnapshotReport,
    StartupOptions, StartupView, UiScale, default_options_path, load_options_file,
    render_operad_snapshot_scaled, run_3d_benchmark_scaled, run_operad_audit_scaled,
    save_options_file,
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
    if launch.list_views {
        print_view_list();
        return Ok(());
    }
    if let Some(path) = &launch.write_default_options {
        save_options_file(path, &AppOptions::default()).map_err(std::io::Error::other)?;
        println!("wrote default options {}", path.display());
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

    let (app_options, options_file_path) =
        load_launch_options_file(&launch).map_err(std::io::Error::other)?;
    if launch.print_options {
        println!(
            "{}",
            app_options
                .to_pretty_json()
                .map_err(std::io::Error::other)?
        );
        return Ok(());
    }

    let mut startup_options = launch.startup_options();
    startup_options.app_options = Some(app_options.clone());
    startup_options.options_file_path = Some(options_file_path.display().to_string());
    let ui_scale = UiScale::new(if launch.ui_scale_overridden {
        launch.ui_scale
    } else {
        app_options.appearance.ui_scale
    });
    if launch.benchmark_3d.is_some() {
        let report = run_3d_benchmark_scaled(startup_options, ui_scale)?;
        println!("{}", report.summary());
    } else if let Some((width, height)) = launch.operad_snapshot {
        let report = render_operad_snapshot_scaled(startup_options, width, height, ui_scale)?;
        if let Some(path) = &launch.snapshot_rgba {
            write_snapshot_rgba(path, &report)?;
        }
        println!("{}", report.summary());
    } else if launch.audit {
        let report = run_operad_audit_scaled(startup_options, ui_scale)?;
        println!("{}", report.summary());
    } else {
        native_window::run(startup_options)?;
    }
    Ok(())
}

fn init_logging() {
    let filter = match std::env::var("RUST_LOG") {
        Ok(value) => tracing_subscriber::EnvFilter::try_new(value).unwrap_or_else(|err| {
            eprintln!("WARN invalid RUST_LOG filter; using default filters: {err}");
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
    list_views: bool,
    audit: bool,
    offscreen: Option<OffscreenRenderOptions>,
    export_gds: Option<PathBuf>,
    benchmark_3d: Option<Benchmark3dLaunch>,
    operad_snapshot: Option<(u32, u32)>,
    snapshot_rgba: Option<PathBuf>,
    ui_scale: f32,
    ui_scale_overridden: bool,
    options_file: Option<PathBuf>,
    write_default_options: Option<PathBuf>,
    print_options: bool,
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
        let mut list_views = false;
        let mut audit = false;
        let mut offscreen_requested = env_offscreen;
        let mut offscreen = OffscreenRenderOptions::default();
        let mut export_gds = None;
        let mut benchmark_3d = None;
        let mut operad_snapshot = None;
        let mut snapshot_rgba = None;
        let mut ui_scale = 1.0;
        let mut ui_scale_overridden = false;
        let mut options_file = None;
        let mut write_default_options = None;
        let mut print_options = false;
        let mut startup_options = StartupOptions::default();
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => help = true,
                "--list-views" => list_views = true,
                "--audit" => audit = true,
                "--offscreen" | "--headless" => offscreen_requested = true,
                "--snapshot" => operad_snapshot = Some((1440, 920)),
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
                "--options-file" => {
                    options_file = Some(PathBuf::from(next_value(&mut args, "--options-file")?));
                }
                "--write-default-options" => {
                    write_default_options = Some(PathBuf::from(next_value(
                        &mut args,
                        "--write-default-options",
                    )?));
                }
                "--print-options" => print_options = true,
                "--click" => {
                    startup_options
                        .startup_actions
                        .push(next_value(&mut args, "--click")?);
                }
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
                "--ui-scale" => {
                    ui_scale = parse_ui_scale(&next_value(&mut args, "--ui-scale")?)?;
                    ui_scale_overridden = true;
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
                "--bench-3d-width" => {
                    let value = next_value(&mut args, "--bench-3d-width")?;
                    benchmark_3d
                        .get_or_insert_with(Benchmark3dLaunch::default)
                        .options
                        .width = parse_u32(&value, "--bench-3d-width")?;
                }
                "--bench-3d-height" => {
                    let value = next_value(&mut args, "--bench-3d-height")?;
                    benchmark_3d
                        .get_or_insert_with(Benchmark3dLaunch::default)
                        .options
                        .height = parse_u32(&value, "--bench-3d-height")?;
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
                    } else if let Some(value) = arg.strip_prefix("--ui-scale=") {
                        ui_scale = parse_ui_scale(value)?;
                        ui_scale_overridden = true;
                    } else if let Some(value) = arg.strip_prefix("--options-file=") {
                        options_file = Some(PathBuf::from(value));
                    } else if let Some(value) = arg.strip_prefix("--write-default-options=") {
                        write_default_options = Some(PathBuf::from(value));
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
                    } else if let Some(value) = arg.strip_prefix("--click=") {
                        startup_options.startup_actions.push(value.to_string());
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
                    } else if let Some(value) = arg.strip_prefix("--bench-3d-width=") {
                        benchmark_3d
                            .get_or_insert_with(Benchmark3dLaunch::default)
                            .options
                            .width = parse_u32(value, "--bench-3d-width")?;
                    } else if let Some(value) = arg.strip_prefix("--bench-3d-height=") {
                        benchmark_3d
                            .get_or_insert_with(Benchmark3dLaunch::default)
                            .options
                            .height = parse_u32(value, "--bench-3d-height")?;
                    } else {
                        return Err(format!("unknown argument {arg:?}; use --help"));
                    }
                }
            }
        }

        Ok(Self {
            help,
            list_views,
            audit,
            offscreen: (offscreen_requested && operad_snapshot.is_none()).then_some(offscreen),
            export_gds,
            benchmark_3d,
            operad_snapshot,
            snapshot_rgba,
            ui_scale,
            ui_scale_overridden,
            options_file,
            write_default_options,
            print_options,
            startup_options,
        })
    }

    fn startup_options(&self) -> StartupOptions {
        let mut options = self.startup_options.clone();
        if let Some(benchmark) = self.benchmark_3d {
            options.stress_count = Some(benchmark.count);
            options.view_3d = true;
            options.benchmark_3d = Some(benchmark.options);
        }
        options
    }
}

fn load_launch_options_file(launch: &LaunchOptions) -> Result<(AppOptions, PathBuf), String> {
    let path = launch
        .options_file
        .clone()
        .unwrap_or_else(default_options_path);
    if path.exists() {
        load_options_file(&path).map(|options| (options, path))
    } else {
        Ok((AppOptions::default(), path))
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
        .ok_or_else(|| format!("unsupported view {value:?}; use a supported view slug"))?;
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

fn parse_ui_scale(value: &str) -> Result<f32, String> {
    let scale = parse_f32(value, "--ui-scale")?;
    if scale > 0.0 {
        Ok(scale)
    } else {
        Err("--ui-scale must be greater than zero".to_string())
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
    let image = report
        .render
        .snapshot
        .as_ref()
        .ok_or_else(|| std::io::Error::other("snapshot render did not produce image pixels"))?;
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
        "Usage:\n  \
         fabricad\n  \
         fabricad --list-views\n  \
         fabricad --audit [--ui-scale SCALE]\n  \
         fabricad --snapshot [--width W] [--height H] [--ui-scale SCALE] [--snapshot-rgba PATH] [--view SLUG] [--click NODE] [--scene demo|hierarchy|stress]\n  \
         fabricad --offscreen [--scene demo|hierarchy|stress] [--count N] [--width W] [--height H] [--zoom Z] [--pan X,Y]\n  \
         fabricad --bench-3d [--bench-3d-count N] [--bench-3d-frames N] [--bench-3d-warmup N] [--bench-3d-width W] [--bench-3d-height H]\n  \
         fabricad --export-gds PATH\n  \
         fabricad --print-options [--options-file PATH]\n  \
         fabricad --write-default-options PATH\n\n\
         The default path opens a native window. Use --audit for the noninteractive summary."
    );
}

fn print_view_list() {
    print!("{}", view_list_text());
}

fn view_list_text() -> String {
    let mut output = String::new();
    for view in StartupView::ALL {
        output.push_str(view.slug());
        output.push('\t');
        output.push_str(view.nav_label());
        output.push('\n');
    }
    output
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
    fn parses_options_file_flags() {
        let launch = LaunchOptions::parse(
            [
                "--options-file".to_string(),
                "target/options.json".to_string(),
                "--print-options".to_string(),
                "--write-default-options=target/default-options.json".to_string(),
            ],
            false,
        )
        .unwrap();

        assert_eq!(
            launch.options_file,
            Some(PathBuf::from("target/options.json"))
        );
        assert!(launch.print_options);
        assert_eq!(
            launch.write_default_options,
            Some(PathBuf::from("target/default-options.json"))
        );
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
                "--bench-3d-width=3840".to_string(),
                "--bench-3d-height".to_string(),
                "2160".to_string(),
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
                    width: 3840,
                    height: 2160,
                },
            })
        );
        assert_eq!(
            launch.startup_options().benchmark_3d,
            Some(Benchmark3dOptions {
                frames: 120,
                warmup_frames: 12,
                width: 3840,
                height: 2160,
            })
        );
    }

    #[test]
    fn parses_snapshot_flag() {
        let launch = LaunchOptions::parse(["--snapshot".to_string()], false).unwrap();
        assert!(!launch.audit);
        assert_eq!(launch.offscreen, None);
        assert_eq!(launch.operad_snapshot, Some((1440, 920)));
        assert_eq!(launch.snapshot_rgba, None);
        assert_eq!(launch.ui_scale, 1.0);
    }

    #[test]
    fn parses_snapshot_scene_view_and_output() {
        let launch = LaunchOptions::parse(
            [
                "--snapshot".to_string(),
                "--scene=hierarchy".to_string(),
                "--ui-scale=2.0".to_string(),
                "--view".to_string(),
                "layout".to_string(),
                "--snapshot-rgba=target/snapshot.rgba".to_string(),
            ],
            false,
        )
        .unwrap();

        assert_eq!(launch.offscreen, None);
        assert_eq!(launch.operad_snapshot, Some((1440, 920)));
        assert_eq!(
            launch.snapshot_rgba,
            Some(PathBuf::from("target/snapshot.rgba"))
        );
        assert_eq!(launch.ui_scale, 2.0);
        assert!(launch.startup_options().hierarchy_demo);
        assert_eq!(
            launch.startup_options().view_mode,
            Some(StartupView::Layout2d)
        );
    }

    #[test]
    fn parses_snapshot_startup_clicks() {
        let launch = LaunchOptions::parse(
            [
                "--snapshot".to_string(),
                "--click".to_string(),
                "fabricad.menu.view".to_string(),
                "--click=fabricad.menu.item.view.group.engineering".to_string(),
            ],
            false,
        )
        .unwrap();

        assert_eq!(
            launch.startup_options().startup_actions,
            vec![
                "fabricad.menu.view".to_string(),
                "fabricad.menu.item.view.group.engineering".to_string()
            ]
        );
    }

    #[test]
    fn parses_audit_flag() {
        let launch = LaunchOptions::parse(["--audit".to_string()], false).unwrap();
        assert!(launch.audit);
        assert!(!launch.list_views);
        assert_eq!(launch.offscreen, None);
        assert_eq!(launch.operad_snapshot, None);
    }

    #[test]
    fn parses_list_views_flag() {
        let launch = LaunchOptions::parse(["--list-views".to_string()], false).unwrap();
        assert!(launch.list_views);
        assert!(!launch.audit);
        assert_eq!(launch.offscreen, None);
        assert_eq!(launch.operad_snapshot, None);
    }

    #[test]
    fn list_views_output_matches_startup_views() {
        let output = view_list_text();
        let lines = output.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), StartupView::ALL.len());
        for (line, view) in lines.iter().zip(StartupView::ALL) {
            assert_eq!(*line, format!("{}\t{}", view.slug(), view.nav_label()));
        }
    }
}

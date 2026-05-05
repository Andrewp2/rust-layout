use fabricad_app::{OffscreenRenderOptions, OffscreenScene};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Fabricad")
            .with_inner_size([1440.0, 920.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "Fabricad",
        options,
        Box::new(|cc| Ok(Box::new(fabricad_app::FabricadApp::new(cc)))),
    )?;
    Ok(())
}

#[derive(Debug, PartialEq)]
struct LaunchOptions {
    help: bool,
    offscreen: Option<OffscreenRenderOptions>,
    export_gds: Option<PathBuf>,
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
        })
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
    }
}

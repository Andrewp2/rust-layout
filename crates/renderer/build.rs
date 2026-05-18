use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn main() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("renderer crate lives under crates/renderer")
        .to_path_buf();
    let slang_root = workspace_root.join("assets/shaders/slang");
    let script = workspace_root.join("scripts/compile_shaders.py");

    println!("cargo:rerun-if-changed={}", slang_root.display());
    println!("cargo:rerun-if-changed={}", script.display());

    if std::env::var_os("GLASSWORKS_SKIP_SHADER_COMPILE").is_some() {
        println!(
            "cargo:warning=Skipping Slang shader compilation because GLASSWORKS_SKIP_SHADER_COMPILE is set"
        );
        return;
    }

    let current = fingerprint_inputs(&slang_root, &script);
    let stamp_path = workspace_root.join("target/glassworks_shader_inputs.fingerprint");
    let previous = read_to_string(&stamp_path).unwrap_or_default();
    let outputs_missing = compiled_outputs_missing(&workspace_root);
    if previous == current && !outputs_missing {
        return;
    }

    let mut formats = vec!["wgsl"];
    if std::env::var("CARGO_CFG_TARGET_ARCH").ok().as_deref() != Some("wasm32") {
        formats.push("spirv");
    }

    for format in formats {
        let status = Command::new("python3")
            .arg(&script)
            .arg(format)
            .current_dir(&workspace_root)
            .status();
        match status {
            Ok(status) if status.success() => {}
            Ok(status) => {
                let message =
                    format!("Slang shader compile failed for {format} with status {status}");
                if std::env::var_os("GLASSWORKS_REQUIRE_SHADER_COMPILE").is_some() {
                    panic!("{message}");
                }
                println!("cargo:warning={message}");
                return;
            }
            Err(err) => {
                let message = format!("failed to launch scripts/compile_shaders.py: {err}");
                if std::env::var_os("GLASSWORKS_REQUIRE_SHADER_COMPILE").is_some() {
                    panic!("{message}");
                }
                println!("cargo:warning={message}");
                return;
            }
        }
    }

    if let Some(parent) = stamp_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = fs::File::create(stamp_path) {
        let _ = file.write_all(current.as_bytes());
    }
}

fn fingerprint_inputs(slang_root: &Path, script: &Path) -> String {
    let mut entries = Vec::new();
    gather_slang_entries(slang_root, &mut entries);
    push_entry(script, &mut entries);
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = DefaultHasher::new();
    entries.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn gather_slang_entries(dir: &Path, out: &mut Vec<(String, u128)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            gather_slang_entries(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("slang") {
            push_entry(&path, out);
        }
    }
}

fn push_entry(path: &Path, out: &mut Vec<(String, u128)>) {
    let ts = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    out.push((path.to_string_lossy().into_owned(), ts));
}

fn compiled_outputs_missing(workspace_root: &Path) -> bool {
    let compiled = workspace_root.join("assets/shaders/compiled_shaders");
    let wgsl_missing = !has_extension(&compiled.join("wgsl"), "wgsl");
    let spirv_missing = !has_extension(&compiled.join("spirv"), "spv");
    wgsl_missing || spirv_missing
}

fn has_extension(dir: &Path, extension: &str) -> bool {
    fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.flatten())
        .any(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == extension)
        })
}

fn read_to_string(path: &Path) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;
    Some(contents)
}

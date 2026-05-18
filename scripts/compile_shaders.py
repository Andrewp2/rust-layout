#!/usr/bin/env python3
import os
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
SLANG_ROOT = ROOT / "assets" / "shaders" / "slang"
OUT_ROOT = ROOT / "assets" / "shaders" / "compiled_shaders"


def find_slangc() -> str:
    override = os.environ.get("GLASSWORKS_SLANGC")
    if override:
        return override
    path_slangc = shutil.which("slangc")
    if path_slangc:
        return path_slangc
    candidates = [
        Path.home() / "tools" / "bin" / "slangc",
        Path.home() / "tools" / "slang-2026.2-linux-x86_64" / "bin" / "slangc",
        Path("/opt/slang-2026.2/bin/slangc"),
        Path("/opt/slang-2025.19.1/bin/slangc"),
    ]
    for candidate in candidates:
        if candidate.is_file():
            return str(candidate)
    raise SystemExit("Could not locate slangc. Set GLASSWORKS_SLANGC or add slangc to PATH.")


def modules() -> list[Path]:
    return sorted(SLANG_ROOT.rglob("*.slang"))


def compile_module(slangc: str, source: Path, target: str) -> None:
    rel = source.relative_to(SLANG_ROOT).with_suffix("")
    output_dir = OUT_ROOT / target
    reflection_dir = OUT_ROOT / "reflection"
    output_dir.mkdir(parents=True, exist_ok=True)
    reflection_dir.mkdir(parents=True, exist_ok=True)
    ext = "wgsl" if target == "wgsl" else "spv"
    output = output_dir / f"{rel.as_posix().replace('/', '_')}.{ext}"
    reflection = reflection_dir / f"{rel.as_posix().replace('/', '_')}.json"
    command = [
        slangc,
        "-I",
        str(SLANG_ROOT),
        str(source),
        "-target",
        "wgsl" if target == "wgsl" else "spirv",
        "-profile",
        "sm_6_0",
        "-fvk-use-entrypoint-name",
        "-reflection-json",
        str(reflection),
        "-o",
        str(output),
    ]
    if target == "spirv":
        command.insert(-2, "-O3")
    subprocess.run(command, cwd=ROOT, check=True)


def clean(target: str) -> None:
    output_dir = OUT_ROOT / target
    output_dir.mkdir(parents=True, exist_ok=True)
    for path in output_dir.iterdir():
        if path.suffix in {".wgsl", ".spv"}:
            path.unlink()


def main(argv: list[str]) -> int:
    target = argv[1] if len(argv) > 1 else "all"
    if target not in {"all", "wgsl", "spirv"}:
        print("usage: scripts/compile_shaders.py [all|wgsl|spirv]", file=sys.stderr)
        return 2
    slangc = find_slangc()
    targets = ["wgsl", "spirv"] if target == "all" else [target]
    discovered = modules()
    if not discovered:
        print(f"No .slang files found under {SLANG_ROOT}", file=sys.stderr)
        return 1
    for selected in targets:
        clean(selected)
        for source in discovered:
            compile_module(slangc, source, selected)
    print(f"Compiled {len(discovered)} Slang module(s) to {', '.join(targets)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))

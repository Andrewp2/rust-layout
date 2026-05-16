#!/usr/bin/env python3
from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Callable

from PIL import Image


ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "target" / "render-e2e"
BASELINE_DIR = ROOT / "tests" / "render_baselines"
DIFF_DIR = OUT_DIR / "diffs"
WIDTH = 1280
HEIGHT = 720
MAX_CHANGED_RATIO = 0.015
MAX_MEAN_DELTA = 1.2
PIXEL_DELTA_THRESHOLD = 12


class BaselineMode(Enum):
    CHECK = "check"
    UPDATE = "update"
    SKIP = "skip"


@dataclass
class Metrics:
    non_dark: int
    blue: int
    green: int
    orange: int
    magenta: int
    teal: int
    bright: int
    sampled_unique: int

    @property
    def layer_pixels(self) -> int:
        return self.blue + self.green + self.orange + self.magenta + self.teal


@dataclass(frozen=True)
class ViewCase:
    slug: str
    label: str


@dataclass(frozen=True)
class Case:
    name: str
    args: tuple[str, ...]
    assertion: Callable[[Metrics], None]
    baseline: bool = False
    image_assertion: Callable[[Image.Image], None] | None = None


@dataclass
class DiffMetrics:
    changed_ratio: float
    mean_delta: float
    max_delta: int


def native_binary() -> Path:
    subprocess.run(
        ["cargo", "build", "-p", "native_app", "--bin", "fabricad"],
        cwd=ROOT,
        check=True,
    )
    binary = ROOT / "target" / "debug" / "fabricad"
    if not binary.exists():
        raise RuntimeError(f"native binary was not built: {binary}")
    return binary


def load_views(binary: Path) -> list[ViewCase]:
    result = subprocess.run(
        [str(binary), "--list-views"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise RuntimeError(f"view list command failed: {result.stderr.strip()}")
    views = []
    for line in result.stdout.splitlines():
        if not line.strip():
            continue
        parts = line.split("\t", 1)
        if len(parts) != 2:
            raise RuntimeError(f"malformed view list line: {line!r}")
        views.append(ViewCase(parts[0], parts[1]))
    if not views:
        raise RuntimeError("view list command returned no views")
    return views


def render_case(binary: Path, case: Case, out_dir: Path, keep_raw: bool) -> Image.Image:
    raw_path = out_dir / f"{case.name}.rgba"
    png_path = out_dir / f"{case.name}.png"
    summary_path = out_dir / f"{case.name}.summary.txt"
    command = [
        str(binary),
        "--snapshot",
        "--width",
        str(WIDTH),
        "--height",
        str(HEIGHT),
        "--snapshot-rgba",
        str(raw_path),
        *case.args,
    ]
    result = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        capture_output=True,
        timeout=90,
    )
    summary_path.write_text(result.stdout + result.stderr, encoding="utf-8")
    if result.returncode != 0:
        raise RuntimeError(
            f"{case.name} snapshot command failed with {result.returncode}; see {summary_path}"
        )

    raw = raw_path.read_bytes()
    expected = WIDTH * HEIGHT * 4
    if len(raw) != expected:
        raise RuntimeError(
            f"{case.name} wrote {len(raw)} RGBA bytes, expected {expected}"
        )
    image = Image.frombytes("RGBA", (WIDTH, HEIGHT), raw).convert("RGB")
    image.save(png_path)
    if not keep_raw:
        raw_path.unlink(missing_ok=True)
    return image


def analyze_image(image: Image.Image) -> Metrics:
    pixels = list(image.getdata())
    sampled = pixels[:: max(1, len(pixels) // 20_000)]
    unique = len(set(sampled))

    non_dark = blue = green = orange = magenta = teal = bright = 0
    for r, g, b in pixels:
        if max(r, g, b) > 45:
            non_dark += 1
        if b > 105 and g > 55 and r < 95:
            blue += 1
        if g > 115 and r < 125 and b < 165:
            green += 1
        if r > 100 and g > 55 and b < 95 and r > b + 30:
            orange += 1
        if r > 95 and b > 75 and g < 95:
            magenta += 1
        if g > 120 and b > 110 and r < 150:
            teal += 1
        if r > 170 and g > 170 and b > 150:
            bright += 1

    return Metrics(
        non_dark=non_dark,
        blue=blue,
        green=green,
        orange=orange,
        magenta=magenta,
        teal=teal,
        bright=bright,
        sampled_unique=unique,
    )


def assert_ui_shell(metrics: Metrics) -> None:
    failures = []
    if metrics.non_dark < 40_000:
        failures.append(f"snapshot is too dark/nonblank pixels={metrics.non_dark}")
    if metrics.bright < 1_000:
        failures.append(f"text and chrome highlights appear missing/bright pixels={metrics.bright}")
    if metrics.teal < 100:
        failures.append(f"accent/status color appears missing/teal pixels={metrics.teal}")
    if metrics.sampled_unique < 30:
        failures.append(f"snapshot has too little visual variation/unique={metrics.sampled_unique}")
    if failures:
        raise AssertionError("; ".join(failures))


def assert_layout_scene(metrics: Metrics) -> None:
    assert_ui_shell(metrics)
    if metrics.layer_pixels < 800:
        raise AssertionError(f"layout preview color coverage is too low/layer pixels={metrics.layer_pixels}")


def assert_dense_scene(metrics: Metrics) -> None:
    assert_ui_shell(metrics)
    if metrics.layer_pixels < 1_200:
        raise AssertionError(f"dense layout preview color coverage is too low/layer pixels={metrics.layer_pixels}")


def assert_layout_canvas_visible(image: Image.Image) -> None:
    canvas = image.crop((220, 60, 748, HEIGHT))
    metrics = analyze_image(canvas)
    failures = []
    if metrics.non_dark < 10_000:
        failures.append(f"canvas is too dark/nonblank pixels={metrics.non_dark}")
    if metrics.layer_pixels < 2_500:
        failures.append(f"canvas layer color coverage is too low/layer pixels={metrics.layer_pixels}")
    if metrics.sampled_unique < 4:
        failures.append(f"canvas has too little visual variation/unique={metrics.sampled_unique}")
    if failures:
        raise AssertionError("; ".join(failures))


def compare_to_baseline(name: str, actual: Image.Image, baseline_dir: Path, diff_dir: Path) -> DiffMetrics:
    baseline_path = baseline_dir / f"{name}.png"
    if not baseline_path.exists():
        raise AssertionError(
            f"missing baseline {baseline_path}; run ./scripts/render_e2e.py --update-baselines"
        )
    baseline = Image.open(baseline_path).convert("RGB")
    if baseline.size != actual.size:
        raise AssertionError(
            f"{name} screenshot size changed: actual={actual.size} baseline={baseline.size}"
        )

    actual_pixels = list(actual.getdata())
    baseline_pixels = list(baseline.getdata())
    diff_image = Image.new("RGB", actual.size)
    diff_pixels = []
    changed = 0
    total_delta = 0
    max_delta = 0
    for actual_rgb, baseline_rgb in zip(actual_pixels, baseline_pixels, strict=True):
        deltas = [abs(a - b) for a, b in zip(actual_rgb, baseline_rgb, strict=True)]
        delta = max(deltas)
        total_delta += sum(deltas) / 3
        max_delta = max(max_delta, delta)
        if delta > PIXEL_DELTA_THRESHOLD:
            changed += 1
            diff_pixels.append((255, min(255, delta * 6), 0))
        else:
            faded = tuple(min(255, int(channel * 0.28)) for channel in actual_rgb)
            diff_pixels.append(faded)
    diff_image.putdata(diff_pixels)
    diff_dir.mkdir(parents=True, exist_ok=True)
    diff_image.save(diff_dir / f"{name}.diff.png")

    pixel_count = len(actual_pixels)
    metrics = DiffMetrics(
        changed_ratio=changed / pixel_count,
        mean_delta=total_delta / pixel_count,
        max_delta=max_delta,
    )
    if metrics.changed_ratio > MAX_CHANGED_RATIO or metrics.mean_delta > MAX_MEAN_DELTA:
        raise AssertionError(
            f"{name} visual diff too large: changed={metrics.changed_ratio:.4%} "
            f"mean_delta={metrics.mean_delta:.3f} max_delta={metrics.max_delta}; "
            f"see {diff_dir / f'{name}.diff.png'}"
        )
    return metrics


def update_baseline(name: str, actual: Image.Image, baseline_dir: Path) -> None:
    baseline_dir.mkdir(parents=True, exist_ok=True)
    actual.save(baseline_dir / f"{name}.png")


def cases(views: list[ViewCase]) -> list[Case]:
    baseline_view_slugs = {
        "workflow",
        "layout",
        "layout2d",
        "3d",
        "layout3d",
        "process-flow",
    }
    baseline_cases = [
        Case("workflow", ("--workspace=demo", "--view=workflow"), assert_ui_shell, True),
        Case(
            "demo",
            ("--workspace=demo", "--view=layout"),
            assert_layout_scene,
            True,
            assert_layout_canvas_visible,
        ),
        Case(
            "hierarchy",
            ("--scene=hierarchy", "--view=layout"),
            assert_layout_scene,
            True,
            assert_layout_canvas_visible,
        ),
        Case(
            "stress_lod",
            ("--scene=stress", "--count=20000", "--view=layout"),
            assert_dense_scene,
            True,
            assert_layout_canvas_visible,
        ),
        Case(
            "view_3d",
            ("--workspace=demo", "--view=3d"),
            assert_layout_scene,
            True,
            assert_layout_canvas_visible,
        ),
        Case("process_flow", ("--workspace=demo", "--view=process-flow"), assert_ui_shell, True),
    ]
    smoke_cases = [
        Case(
            view.slug.replace("-", "_"),
            ("--workspace=demo", f"--view={view.slug}"),
            assert_ui_shell,
        )
        for view in views
        if view.slug not in baseline_view_slugs
    ]
    return baseline_cases + smoke_cases


def main() -> int:
    parser = argparse.ArgumentParser(description="Run screenshot render E2E tests.")
    parser.add_argument("--keep-artifacts", action="store_true")
    baseline_group = parser.add_mutually_exclusive_group()
    baseline_group.add_argument(
        "--update-baselines",
        action="store_true",
        help="write current screenshots to tests/render_baselines instead of comparing",
    )
    baseline_group.add_argument(
        "--check-baseline",
        action="store_true",
        help="compare screenshots against quarantined whole-UI baselines",
    )
    baseline_group.add_argument(
        "--skip-baseline",
        action="store_true",
        help="run semantic screenshot checks without baseline pixel comparison (default)",
    )
    args = parser.parse_args()
    baseline_mode = (
        BaselineMode.UPDATE
        if args.update_baselines
        else BaselineMode.CHECK
        if args.check_baseline
        else BaselineMode.SKIP
    )

    if OUT_DIR.exists():
        shutil.rmtree(OUT_DIR)
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    DIFF_DIR.mkdir(parents=True, exist_ok=True)
    binary = native_binary()
    view_list = load_views(binary)

    for case in cases(view_list):
        image = render_case(binary, case, OUT_DIR, args.keep_artifacts)
        metrics = analyze_image(image)
        case.assertion(metrics)
        if case.image_assertion is not None:
            case.image_assertion(image)
        diff = None
        if baseline_mode == BaselineMode.UPDATE and case.baseline:
            update_baseline(case.name, image, BASELINE_DIR)
        elif baseline_mode == BaselineMode.CHECK and case.baseline:
            diff = compare_to_baseline(case.name, image, BASELINE_DIR, DIFF_DIR)
        suffix = f" diff={diff}" if diff else ""
        print(f"{case.name}: {metrics}{suffix}")

    print(f"render E2E artifacts: {OUT_DIR}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

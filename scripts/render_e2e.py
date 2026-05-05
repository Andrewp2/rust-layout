#!/usr/bin/env python3
from __future__ import annotations

import argparse
import base64
import contextlib
import io
import json
import os
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Any

import requests
import websocket
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
    bright: int
    sampled_unique: int

    @property
    def layer_pixels(self) -> int:
        return self.blue + self.green + self.orange + self.magenta


@dataclass
class Case:
    name: str
    query: str
    assertion: Any


@dataclass
class DiffMetrics:
    changed_ratio: float
    mean_delta: float
    max_delta: int


class Cdp:
    def __init__(self, ws_url: str):
        self.ws = websocket.create_connection(ws_url, timeout=10)
        self.next_id = 1

    def close(self) -> None:
        self.ws.close()

    def call(self, method: str, params: dict[str, Any] | None = None) -> dict[str, Any]:
        msg_id = self.next_id
        self.next_id += 1
        self.ws.send(json.dumps({"id": msg_id, "method": method, "params": params or {}}))
        while True:
            message = json.loads(self.ws.recv())
            if message.get("id") == msg_id:
                if "error" in message:
                    raise RuntimeError(f"CDP {method} failed: {message['error']}")
                return message.get("result", {})


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def wait_http(url: str, timeout_s: float) -> None:
    deadline = time.time() + timeout_s
    last_error: Exception | None = None
    while time.time() < deadline:
        try:
            response = requests.get(url, timeout=1)
            if response.status_code < 500:
                return
        except Exception as error:
            last_error = error
        time.sleep(0.25)
    raise RuntimeError(f"timed out waiting for {url}: {last_error}")


def start_trunk(port: int, out_dir: Path) -> subprocess.Popen:
    log = open(out_dir / "trunk.log", "w", encoding="utf-8")
    env = os.environ.copy()
    env.pop("NO_COLOR", None)
    return subprocess.Popen(
        ["trunk", "serve", "--address", "127.0.0.1", "--port", str(port)],
        cwd=ROOT,
        env=env,
        stdout=log,
        stderr=subprocess.STDOUT,
        text=True,
    )


def chrome_binary() -> str:
    override = os.environ.get("CHROME")
    if override:
        return override
    for name in ("google-chrome", "chromium", "chromium-browser"):
        path = shutil.which(name)
        if path:
            return path
    raise RuntimeError("could not find Chrome/Chromium; set CHROME=/path/to/chrome")


def start_chrome(debug_port: int, profile: Path) -> subprocess.Popen:
    log = open(OUT_DIR / "chrome.log", "w", encoding="utf-8")
    command = [
        chrome_binary(),
        "--headless=new",
        "--no-sandbox",
        "--disable-dev-shm-usage",
        "--disable-gpu-sandbox",
        "--disable-extensions",
        "--enable-unsafe-swiftshader",
        "--enable-unsafe-webgpu",
        "--ignore-gpu-blocklist",
        "--use-angle=swiftshader",
        "--remote-allow-origins=*",
        f"--remote-debugging-port={debug_port}",
        f"--user-data-dir={profile}",
        f"--window-size={WIDTH},{HEIGHT}",
        "about:blank",
    ]
    return subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT, text=True)


def connect_cdp(debug_port: int) -> Cdp:
    version_url = f"http://127.0.0.1:{debug_port}/json"
    deadline = time.time() + 10
    while time.time() < deadline:
        try:
            targets = requests.get(version_url, timeout=1).json()
            page = next(target for target in targets if target.get("type") == "page")
            return Cdp(page["webSocketDebuggerUrl"])
        except Exception:
            time.sleep(0.2)
    raise RuntimeError("timed out waiting for Chrome DevTools Protocol")


def wait_for_app(cdp: Cdp) -> None:
    deadline = time.time() + 30
    while time.time() < deadline:
        result = cdp.call(
            "Runtime.evaluate",
            {
                "returnByValue": True,
                "expression": """
                    (() => {
                        const canvas = document.getElementById('fabricad_canvas');
                        const rect = canvas ? canvas.getBoundingClientRect() : { width: 0, height: 0 };
                        return document.readyState === 'complete'
                            && !!canvas
                            && rect.width >= 300
                            && rect.height >= 300;
                    })()
                """,
            },
        )
        if result.get("result", {}).get("value"):
            time.sleep(1.0)
            return
        time.sleep(0.2)
    raise RuntimeError("timed out waiting for Fabricad canvas")


def capture_png(cdp: Cdp, path: Path) -> Image.Image:
    screenshot = cdp.call("Page.captureScreenshot", {"format": "png", "fromSurface": True})
    data = base64.b64decode(screenshot["data"])
    path.write_bytes(data)
    return Image.open(io.BytesIO(data)).convert("RGB")


def analyze_canvas_region(image: Image.Image) -> Metrics:
    # Ignore toolbar/sidebar chrome so assertions describe the rendered canvas content.
    left = min(300, image.width // 3)
    top = min(54, image.height // 5)
    crop = image.crop((left, top, image.width, image.height))
    pixels = list(crop.getdata())
    sampled = pixels[:: max(1, len(pixels) // 20_000)]
    unique = len(set(sampled))

    non_dark = blue = green = orange = magenta = bright = 0
    for r, g, b in pixels:
        if max(r, g, b) > 45:
            non_dark += 1
        if b > 105 and g > 55 and r < 90:
            blue += 1
        if g > 115 and r < 110 and b < 155:
            green += 1
        if r > 80 and g > 45 and b < 75 and r > b + 30:
            orange += 1
        if r > 85 and b > 55 and g < 80:
            magenta += 1
        if r > 170 and g > 170 and b > 150:
            bright += 1

    return Metrics(
        non_dark=non_dark,
        blue=blue,
        green=green,
        orange=orange,
        magenta=magenta,
        bright=bright,
        sampled_unique=unique,
    )


def assert_demo(metrics: Metrics) -> None:
    failures = []
    if metrics.non_dark < 12_000:
        failures.append(f"canvas is too dark/nonblank pixels={metrics.non_dark}")
    if metrics.blue < 500:
        failures.append(f"metal-blue layer appears missing/blue pixels={metrics.blue}")
    if metrics.green < 500:
        failures.append(f"diffusion-green layer appears missing/green pixels={metrics.green}")
    if metrics.orange < 200:
        failures.append(f"poly-orange layer appears missing/orange pixels={metrics.orange}")
    if metrics.sampled_unique < 40:
        failures.append(f"image has too little visual variation/unique={metrics.sampled_unique}")
    if failures:
        raise AssertionError("; ".join(failures))


def assert_stress_lod(metrics: Metrics) -> None:
    failures = []
    if metrics.non_dark < 30_000:
        failures.append(f"stress canvas is too sparse/nonblank pixels={metrics.non_dark}")
    if metrics.layer_pixels < 8_000:
        failures.append(f"stress layer color coverage is too low/layer pixels={metrics.layer_pixels}")
    if metrics.sampled_unique < 10:
        failures.append(f"stress image has too little variation/unique={metrics.sampled_unique}")
    if failures:
        raise AssertionError("; ".join(failures))


def assert_hierarchy(metrics: Metrics) -> None:
    failures = []
    if metrics.non_dark < 18_000:
        failures.append(f"hierarchy canvas is too sparse/nonblank pixels={metrics.non_dark}")
    if metrics.blue < 800:
        failures.append(f"hierarchy metal-blue layer appears missing/blue pixels={metrics.blue}")
    if metrics.green < 800:
        failures.append(f"hierarchy diffusion-green layer appears missing/green pixels={metrics.green}")
    if metrics.orange < 400:
        failures.append(f"hierarchy poly-orange layer appears missing/orange pixels={metrics.orange}")
    if metrics.magenta < 200:
        failures.append(f"hierarchy metal2-magenta layer appears missing/magenta pixels={metrics.magenta}")
    if metrics.sampled_unique < 35:
        failures.append(f"hierarchy image has too little visual variation/unique={metrics.sampled_unique}")
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


def run_case(
    cdp: Cdp,
    base_url: str,
    case: Case,
    out_dir: Path,
    baseline_dir: Path,
    baseline_mode: BaselineMode,
) -> tuple[Metrics, DiffMetrics | None]:
    url = f"{base_url}/{case.query}"
    cdp.call("Page.navigate", {"url": url})
    wait_for_app(cdp)

    last_metrics: Metrics | None = None
    last_image: Image.Image | None = None
    screenshot_path = out_dir / f"{case.name}.png"
    for _ in range(12):
        image = capture_png(cdp, screenshot_path)
        metrics = analyze_canvas_region(image)
        last_metrics = metrics
        last_image = image
        try:
            case.assertion(metrics)
            break
        except AssertionError:
            time.sleep(0.5)
    assert last_metrics is not None
    assert last_image is not None
    case.assertion(last_metrics)

    diff_metrics = None
    if baseline_mode == BaselineMode.UPDATE:
        update_baseline(case.name, last_image, baseline_dir)
    elif baseline_mode == BaselineMode.CHECK:
        diff_metrics = compare_to_baseline(case.name, last_image, baseline_dir, DIFF_DIR)
    return last_metrics, diff_metrics


def run_cases_once(
    debug_port: int,
    base_url: str,
    cases: list[Case],
    baseline_mode: BaselineMode,
) -> None:
    chrome_profile = Path(tempfile.mkdtemp(prefix="fabricad-chrome-", dir=OUT_DIR))
    chrome: subprocess.Popen | None = None
    cdp: Cdp | None = None
    try:
        chrome = start_chrome(debug_port, chrome_profile)
        cdp = connect_cdp(debug_port)
        cdp.call("Page.enable")
        cdp.call("Runtime.enable")
        cdp.call(
            "Emulation.setDeviceMetricsOverride",
            {
                "width": WIDTH,
                "height": HEIGHT,
                "deviceScaleFactor": 1,
                "mobile": False,
            },
        )

        for case in cases:
            metrics, diff = run_case(cdp, base_url, case, OUT_DIR, BASELINE_DIR, baseline_mode)
            suffix = f" diff={diff}" if diff else ""
            print(f"{case.name}: {metrics}{suffix}")
    finally:
        if cdp:
            with contextlib.suppress(Exception):
                cdp.close()
        if chrome:
            terminate(chrome)
        with contextlib.suppress(Exception):
            shutil.rmtree(chrome_profile)


def terminate(process: subprocess.Popen) -> None:
    if process.poll() is not None:
        return
    with contextlib.suppress(Exception):
        process.send_signal(signal.SIGTERM)
        process.wait(timeout=5)
    if process.poll() is None:
        with contextlib.suppress(Exception):
            process.kill()


def main() -> int:
    parser = argparse.ArgumentParser(description="Run Fabricad screenshot render E2E tests.")
    parser.add_argument("--keep-artifacts", action="store_true")
    parser.add_argument(
        "--update-baselines",
        action="store_true",
        help="write current screenshots to tests/render_baselines instead of comparing",
    )
    parser.add_argument(
        "--skip-baseline",
        action="store_true",
        help="run semantic screenshot checks without baseline pixel comparison",
    )
    args = parser.parse_args()
    if args.update_baselines and args.skip_baseline:
        parser.error("--update-baselines and --skip-baseline are mutually exclusive")
    baseline_mode = (
        BaselineMode.UPDATE
        if args.update_baselines
        else BaselineMode.SKIP
        if args.skip_baseline
        else BaselineMode.CHECK
    )

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    DIFF_DIR.mkdir(parents=True, exist_ok=True)
    port = free_port()
    debug_port = free_port()
    while debug_port == port:
        debug_port = free_port()
    base_url = f"http://127.0.0.1:{port}"
    trunk = start_trunk(port, OUT_DIR)

    try:
        wait_http(base_url, 60)
        cases = [
            Case("demo", "?zoom=0.075", assert_demo),
            Case("selected_handles", "?zoom=0.075&select=first", assert_demo),
            Case("vertex_moved", "?zoom=0.075&edit=vertex_moved", assert_demo),
            Case(
                "hierarchy_workflow",
                "?zoom=0.055&workflow=hierarchy_make_place",
                assert_demo,
            ),
            Case("stress_lod", "?scene=stress&count=20000&zoom=0.008", assert_stress_lod),
            Case("hierarchy", "?scene=hierarchy&zoom=0.045", assert_hierarchy),
        ]
        last_error: Exception | None = None
        for attempt in range(1, 4):
            try:
                run_cases_once(debug_port, base_url, cases, baseline_mode)
                last_error = None
                break
            except (AssertionError, RuntimeError, websocket.WebSocketException) as error:
                last_error = error
                print(f"render E2E browser attempt {attempt} failed: {error}", file=sys.stderr)
                time.sleep(0.75)
        if last_error is not None:
            raise last_error

        print(f"render E2E artifacts: {OUT_DIR}")
        return 0
    finally:
        terminate(trunk)


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
from __future__ import annotations

import argparse
import base64
import contextlib
import html
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
from pathlib import Path
from typing import Any

import requests
import websocket
from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "target" / "ui-layout-audit"


@dataclass(frozen=True)
class ViewCase:
    slug: str
    label: str


@dataclass(frozen=True)
class ViewportSize:
    name: str
    width: int
    height: int


VIEWS = [
    ViewCase("workflow", "Workflow"),
    ViewCase("layout", "Mask layout"),
    ViewCase("3d", "3D viewport"),
    ViewCase("reticle", "Reticle prep"),
    ViewCase("diff", "Layout diff"),
    ViewCase("fab", "Equipment"),
    ViewCase("inventory", "Inventory"),
    ViewCase("maintenance", "Maintenance"),
    ViewCase("environment", "Environment"),
    ViewCase("dispatch", "Dispatch"),
    ViewCase("safety", "Safety"),
    ViewCase("traceability", "Traceability"),
    ViewCase("metrology", "Metrology"),
    ViewCase("yield", "Yield"),
    ViewCase("spc", "SPC / FDC"),
    ViewCase("process-flow", "Process flow"),
    ViewCase("r2r", "R2R control"),
    ViewCase("cross-section", "Cross-section"),
    ViewCase("doe", "DOE"),
    ViewCase("notebook", "Notebook"),
]

SIZES = [
    ViewportSize("narrow", 480, 900),
    ViewportSize("tablet", 768, 1024),
    ViewportSize("laptop", 1280, 720),
    ViewportSize("desktop", 1440, 920),
    ViewportSize("wide", 1920, 1080),
]

QUICK_VIEWS = {"workflow", "layout", "3d", "reticle", "diff", "maintenance", "environment"}
QUICK_SIZES = {"narrow", "laptop", "wide"}


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


def start_chrome(debug_port: int, profile: Path, out_dir: Path) -> subprocess.Popen:
    log = open(out_dir / "chrome.log", "w", encoding="utf-8")
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


def wait_for_app(cdp: Cdp, width: int, height: int) -> None:
    deadline = time.time() + 45
    min_width = min(width, 300)
    min_height = min(height, 300)
    while time.time() < deadline:
        result = cdp.call(
            "Runtime.evaluate",
            {
                "returnByValue": True,
                "expression": f"""
                    (() => {{
                        const canvas = document.getElementById('fabricad_canvas');
                        const rect = canvas ? canvas.getBoundingClientRect() : {{ width: 0, height: 0 }};
                        return document.readyState === 'complete'
                            && !!canvas
                            && rect.width >= {min_width}
                            && rect.height >= {min_height};
                    }})()
                """,
            },
        )
        if result.get("result", {}).get("value"):
            time.sleep(0.8)
            return
        time.sleep(0.2)
    raise RuntimeError("timed out waiting for Fabricad canvas")


def set_viewport(cdp: Cdp, size: ViewportSize) -> None:
    cdp.call(
        "Emulation.setDeviceMetricsOverride",
        {
            "width": size.width,
            "height": size.height,
            "deviceScaleFactor": 1,
            "mobile": False,
        },
    )


def capture_png(cdp: Cdp, path: Path) -> Image.Image:
    screenshot = cdp.call("Page.captureScreenshot", {"format": "png", "fromSurface": True})
    data = base64.b64decode(screenshot["data"])
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    return Image.open(io.BytesIO(data)).convert("RGB")


def make_contact_sheet(
    size: ViewportSize,
    views: list[ViewCase],
    screenshots: dict[tuple[str, str], Path],
    out_dir: Path,
) -> Path:
    thumb_width = 360
    thumb_height = round(thumb_width * size.height / size.width)
    label_height = 28
    gap = 18
    margin = 24
    columns = 3 if size.width < size.height else 4
    rows = (len(views) + columns - 1) // columns
    sheet_width = margin * 2 + columns * thumb_width + (columns - 1) * gap
    sheet_height = margin * 2 + rows * (thumb_height + label_height) + (rows - 1) * gap
    sheet = Image.new("RGB", (sheet_width, sheet_height), (30, 30, 30))
    draw = ImageDraw.Draw(sheet)
    font = ImageFont.load_default()

    for index, view in enumerate(views):
        row = index // columns
        column = index % columns
        x = margin + column * (thumb_width + gap)
        y = margin + row * (thumb_height + label_height + gap)
        path = screenshots[(size.name, view.slug)]
        image = Image.open(path).convert("RGB")
        thumb = image.resize((thumb_width, thumb_height), Image.Resampling.LANCZOS)
        sheet.paste(thumb, (x, y + label_height))
        draw.rectangle(
            [x, y + label_height, x + thumb_width - 1, y + label_height + thumb_height - 1],
            outline=(95, 95, 95),
        )
        draw.text(
            (x, y + 7),
            f"{view.label}  {size.width}x{size.height}",
            fill=(235, 235, 235),
            font=font,
        )

    path = out_dir / "contact-sheets" / f"{size.name}.png"
    path.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(path)
    return path


def write_review_html(
    sizes: list[ViewportSize],
    views: list[ViewCase],
    screenshots: dict[tuple[str, str], Path],
    contact_sheets: list[Path],
    out_dir: Path,
) -> Path:
    rows = []
    for size in sizes:
        cells = []
        for view in views:
            path = screenshots[(size.name, view.slug)]
            rel = path.relative_to(out_dir)
            cells.append(
                f'<figure><a href="{html.escape(str(rel))}">'
                f'<img src="{html.escape(str(rel))}" loading="lazy"></a>'
                f"<figcaption>{html.escape(view.label)}</figcaption></figure>"
            )
        rows.append(
            f"<section><h2>{html.escape(size.name)} {size.width}x{size.height}</h2>"
            f'<div class="grid">{"".join(cells)}</div></section>'
        )

    sheet_links = "".join(
        f'<li><a href="{html.escape(str(path.relative_to(out_dir)))}">'
        f"{html.escape(path.stem)}</a></li>"
        for path in contact_sheets
    )
    html_text = f"""<!doctype html>
<meta charset="utf-8">
<title>Fabricad UI Layout Audit</title>
<style>
body {{
  background: #151718;
  color: #e6e6e6;
  font: 14px system-ui, sans-serif;
  margin: 24px;
}}
a {{ color: #8cc8ff; }}
h1, h2 {{ font-weight: 600; }}
.grid {{
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 18px;
}}
figure {{
  margin: 0;
  background: #222;
  border: 1px solid #444;
  padding: 8px;
}}
img {{
  display: block;
  width: 100%;
  height: auto;
}}
figcaption {{
  color: #cfcfcf;
  margin-top: 6px;
}}
</style>
<h1>Fabricad UI Layout Audit</h1>
<p>Review each screenshot for clipped labels, bad wrapping, floating scrollbars, crowded margins, and content that disappears at narrow or wide aspect ratios.</p>
<h2>Contact sheets</h2>
<ul>{sheet_links}</ul>
{"".join(rows)}
"""
    path = out_dir / "index.html"
    path.write_text(html_text, encoding="utf-8")
    return path


def terminate(process: subprocess.Popen) -> None:
    if process.poll() is not None:
        return
    with contextlib.suppress(Exception):
        process.send_signal(signal.SIGTERM)
        process.wait(timeout=5)
    if process.poll() is None:
        with contextlib.suppress(Exception):
            process.kill()


def selected_views(values: list[str], quick: bool) -> list[ViewCase]:
    if values:
        wanted = set(values)
        unknown = wanted - {view.slug for view in VIEWS}
        if unknown:
            raise ValueError(f"unknown view slug(s): {', '.join(sorted(unknown))}")
        return [view for view in VIEWS if view.slug in wanted]
    if quick:
        return [view for view in VIEWS if view.slug in QUICK_VIEWS]
    return VIEWS


def selected_sizes(values: list[str], quick: bool) -> list[ViewportSize]:
    if values:
        wanted = set(values)
        unknown = wanted - {size.name for size in SIZES}
        if unknown:
            raise ValueError(f"unknown size name(s): {', '.join(sorted(unknown))}")
        return [size for size in SIZES if size.name in wanted]
    if quick:
        return [size for size in SIZES if size.name in QUICK_SIZES]
    return SIZES


def run_audit(views: list[ViewCase], sizes: list[ViewportSize], out_dir: Path) -> None:
    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    port = free_port()
    debug_port = free_port()
    while debug_port == port:
        debug_port = free_port()

    base_url = f"http://127.0.0.1:{port}"
    trunk = start_trunk(port, out_dir)
    chrome_profile = Path(tempfile.mkdtemp(prefix="fabricad-chrome-", dir=out_dir))
    chrome: subprocess.Popen | None = None
    cdp: Cdp | None = None
    screenshots: dict[tuple[str, str], Path] = {}
    try:
        wait_http(base_url, 60)
        chrome = start_chrome(debug_port, chrome_profile, out_dir)
        cdp = connect_cdp(debug_port)
        cdp.call("Page.enable")
        cdp.call("Runtime.enable")
        for size in sizes:
            set_viewport(cdp, size)
            for view in views:
                url = f"{base_url}/?workspace=demo&view={view.slug}"
                cdp.call("Page.navigate", {"url": url})
                wait_for_app(cdp, size.width, size.height)
                path = out_dir / "screenshots" / size.name / f"{view.slug}.png"
                capture_png(cdp, path)
                screenshots[(size.name, view.slug)] = path
                print(f"captured {size.name}/{view.slug}")

        contact_sheets = [
            make_contact_sheet(size, views, screenshots, out_dir) for size in sizes
        ]
        review = write_review_html(sizes, views, screenshots, contact_sheets, out_dir)
        print(f"review: {review}")
    finally:
        if cdp:
            with contextlib.suppress(Exception):
                cdp.close()
        if chrome:
            terminate(chrome)
        terminate(trunk)
        with contextlib.suppress(Exception):
            shutil.rmtree(chrome_profile)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Capture Fabricad whole-app screenshots across views and aspect ratios."
    )
    parser.add_argument("--quick", action="store_true", help="capture a smaller smoke matrix")
    parser.add_argument(
        "--view",
        action="append",
        default=[],
        help="view slug to capture; may be repeated",
    )
    parser.add_argument(
        "--size",
        action="append",
        default=[],
        help="size name to capture; may be repeated",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=OUT_DIR,
        help=f"artifact directory (default: {OUT_DIR})",
    )
    args = parser.parse_args()

    try:
        views = selected_views(args.view, args.quick)
        sizes = selected_sizes(args.size, args.quick)
        run_audit(views, sizes, args.out)
    except Exception as error:
        print(f"ui layout audit failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

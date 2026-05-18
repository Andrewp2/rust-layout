#!/usr/bin/env python3
from __future__ import annotations

import argparse
import html
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

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
    scale: float = 1.0

    def label(self) -> str:
        suffix = "" if self.scale == 1.0 else f" @{self.scale:g}x"
        return f"{self.width}x{self.height}{suffix}"


SIZES = [
    ViewportSize("narrow", 480, 900),
    ViewportSize("tablet", 768, 1024),
    ViewportSize("laptop", 1280, 720),
    ViewportSize("desktop", 1440, 920),
    ViewportSize("wide", 1920, 1080),
    ViewportSize("hidpi", 2048, 1440, 2.0),
]

QUICK_SIZES = {"narrow", "laptop", "wide"}
SUMMARY_COUNT_PATTERN = re.compile(r"\b(paint_items|layout_warnings)=(\d+)\b")
PRESET_CLICKS = {
    "bookmarks-menu": [
        "glassworks.menu.bookmarks",
    ],
    "canvas-2d": [],
    "canvas-3d": [],
    "command-palette": [
        "glassworks.menu.view",
        "glassworks.menu.item.view.command_palette",
    ],
    "file-menu": [
        "glassworks.menu.file",
    ],
    "help-menu": [
        "glassworks.menu.help",
    ],
    "edit-menu": [
        "glassworks.menu.edit",
    ],
    "display-menu": [
        "glassworks.menu.display",
    ],
    "details-panel": [
        "glassworks.menu.display",
        "glassworks.menu.item.display.inspector",
    ],
    "macros-menu": [
        "glassworks.menu.macros",
    ],
    "more-menu": [
        "glassworks.menu.more",
    ],
    "options-panel": [
        "glassworks.menu.options",
        "glassworks.menu.item.display.options",
    ],
    "secondary-panel": [
        "glassworks.menu.display",
        "glassworks.menu.item.display.secondary_panel",
    ],
    "sidebar-modules": [
        "glassworks.menu.view",
        "glassworks.menu.item.view.sidebar_modules",
    ],
    "tools-menu": [
        "glassworks.menu.tools",
    ],
    "view-analysis": [
        "glassworks.menu.view",
        "glassworks.menu.item.view.group.analysis",
    ],
    "view-design": [
        "glassworks.menu.view",
        "glassworks.menu.item.view.group.design",
    ],
    "view-engineering": [
        "glassworks.menu.view",
        "glassworks.menu.item.view.group.engineering",
    ],
    "view-operations": [
        "glassworks.menu.view",
        "glassworks.menu.item.view.group.operations",
    ],
}
PRESET_DEFAULT_VIEWS = {
    "canvas-2d": ["layout2d"],
    "canvas-3d": ["layout3d"],
    "edit-menu": ["layout2d"],
    "secondary-panel": ["layout2d"],
    "tools-menu": ["layout2d"],
}


def preset_default_views(name: str) -> list[str]:
    return PRESET_DEFAULT_VIEWS.get(name, ["workflow"])


def native_binary() -> Path:
    subprocess.run(
        ["cargo", "build", "-p", "native_app", "--bin", "glassworks"],
        cwd=ROOT,
        check=True,
    )
    binary = ROOT / "target" / "debug" / "glassworks"
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


def parse_summary_counts(summary_path: Path) -> dict[str, int]:
    text = summary_path.read_text(encoding="utf-8")
    counts = {key: int(value) for key, value in SUMMARY_COUNT_PATTERN.findall(text)}
    for key in ["paint_items", "layout_warnings"]:
        if key not in counts:
            raise RuntimeError(f"snapshot summary missing {key}: {summary_path}")
    return counts


def click_suffix(clicks: list[str]) -> str:
    if not clicks:
        return ""
    parts = []
    for click in clicks:
        part = re.sub(r"[^A-Za-z0-9]+", "-", click).strip("-").lower()
        part = re.sub(r"^(glassworks-)?menu-item-", "", part)
        part = re.sub(r"^(glassworks-)?menu-", "menu-", part)
        parts.append(part[:48] or "click")
    return "__" + "__".join(parts)[:120]


def capture_snapshot(
    binary: Path,
    view: ViewCase,
    size: ViewportSize,
    out_dir: Path,
    clicks: list[str],
) -> Path:
    stem = f"{view.slug}{click_suffix(clicks)}"
    raw_path = out_dir / "raw" / size.name / f"{stem}.rgba"
    png_path = out_dir / "screenshots" / size.name / f"{stem}.png"
    summary_path = out_dir / "summaries" / size.name / f"{stem}.txt"
    raw_path.parent.mkdir(parents=True, exist_ok=True)
    png_path.parent.mkdir(parents=True, exist_ok=True)
    summary_path.parent.mkdir(parents=True, exist_ok=True)
    command = [
        str(binary),
        "--snapshot",
        "--workspace=demo",
        "--view",
        view.slug,
        "--width",
        str(size.width),
        "--height",
        str(size.height),
        "--snapshot-rgba",
        str(raw_path),
    ]
    if size.scale != 1.0:
        command.extend(["--ui-scale", f"{size.scale:g}"])
    for click in clicks:
        command.extend(["--click", click])
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
            f"snapshot failed for {size.name}/{view.slug} with {result.returncode}; see {summary_path}"
        )
    counts = parse_summary_counts(summary_path)
    if counts["paint_items"] <= 0:
        raise RuntimeError(f"snapshot painted no UI items for {size.name}/{view.slug}; see {summary_path}")
    if counts["layout_warnings"] > 0:
        raise RuntimeError(
            f"snapshot reported {counts['layout_warnings']} layout warning(s) for {size.name}/{view.slug}; see {summary_path}"
        )
    raw = raw_path.read_bytes()
    expected = size.width * size.height * 4
    if len(raw) != expected:
        raise RuntimeError(
            f"snapshot for {size.name}/{view.slug} wrote {len(raw)} RGBA bytes, expected {expected}"
        )
    Image.frombytes("RGBA", (size.width, size.height), raw).convert("RGB").save(png_path)
    raw_path.unlink(missing_ok=True)
    return png_path


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
            f"{view.label}  {size.label()}",
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
            f"<section><h2>{html.escape(size.name)} {html.escape(size.label())}</h2>"
            f'<div class="grid">{"".join(cells)}</div></section>'
        )

    sheet_links = "".join(
        f'<li><a href="{html.escape(str(path.relative_to(out_dir)))}">'
        f"{html.escape(path.stem)}</a></li>"
        for path in contact_sheets
    )
    html_text = f"""<!doctype html>
<meta charset="utf-8">
<title>UI Layout Audit</title>
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
<h1>UI Layout Audit</h1>
<p>Review each snapshot for clipped labels, bad wrapping, crowded margins, and content that disappears at narrow or wide aspect ratios.</p>
<h2>Contact sheets</h2>
<ul>{sheet_links}</ul>
{"".join(rows)}
"""
    path = out_dir / "index.html"
    path.write_text(html_text, encoding="utf-8")
    return path


def write_preset_index(preset_reviews: dict[str, Path], out_dir: Path) -> Path:
    links = "".join(
        f'<li><a href="{html.escape(str(path.relative_to(out_dir)))}">'
        f"{html.escape(name)}</a></li>"
        for name, path in sorted(preset_reviews.items())
    )
    html_text = f"""<!doctype html>
<meta charset="utf-8">
<title>UI Preset Layout Audit</title>
<style>
body {{
  background: #151718;
  color: #e6e6e6;
  font: 14px system-ui, sans-serif;
  margin: 24px;
}}
a {{ color: #8cc8ff; }}
h1 {{ font-weight: 600; }}
</style>
<h1>UI Preset Layout Audit</h1>
<p>Named interactive states captured with startup click sequences.</p>
<ul>{links}</ul>
"""
    path = out_dir / "index.html"
    path.write_text(html_text, encoding="utf-8")
    return path


def selected_views(values: list[str], quick: bool, views: list[ViewCase]) -> list[ViewCase]:
    if values:
        wanted = set(values)
        unknown = wanted - {view.slug for view in views}
        if unknown:
            raise ValueError(f"unknown view slug(s): {', '.join(sorted(unknown))}")
        return [view for view in views if view.slug in wanted]
    if quick:
        return views
    return views


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


def run_audit(
    views: list[ViewCase],
    sizes: list[ViewportSize],
    out_dir: Path,
    binary: Path,
    clicks: list[str],
) -> None:
    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    screenshots: dict[tuple[str, str], Path] = {}

    for size in sizes:
        for view in views:
            screenshots[(size.name, view.slug)] = capture_snapshot(
                binary, view, size, out_dir, clicks
            )
            print(f"captured {size.name}/{view.slug}")

    contact_sheets = [make_contact_sheet(size, views, screenshots, out_dir) for size in sizes]
    review = write_review_html(sizes, views, screenshots, contact_sheets, out_dir)
    print(f"review: {review}")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Capture UI snapshots across views and aspect ratios."
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
    parser.add_argument(
        "--click",
        action="append",
        default=[],
        help="node name to click before each snapshot; may be repeated",
    )
    parser.add_argument(
        "--preset",
        choices=sorted(PRESET_CLICKS),
        help="named interactive state to capture; adds its click sequence",
    )
    parser.add_argument(
        "--all-presets",
        action="store_true",
        help="capture every named interactive preset into subdirectories under --out",
    )
    args = parser.parse_args()

    try:
        if args.all_presets and args.preset:
            raise ValueError("--all-presets cannot be combined with --preset")
        binary = native_binary()
        available_views = load_views(binary)
        base_clicks = list(args.click)
        view_args = list(args.view)
        sizes = selected_sizes(args.size, args.quick)
        if args.all_presets:
            if args.out.exists():
                shutil.rmtree(args.out)
            args.out.mkdir(parents=True, exist_ok=True)
            preset_reviews = {}
            for preset_name in sorted(PRESET_CLICKS):
                preset_views = selected_views(
                    view_args or preset_default_views(preset_name),
                    args.quick,
                    available_views,
                )
                preset_out = args.out / preset_name
                run_audit(
                    preset_views,
                    sizes,
                    preset_out,
                    binary,
                    PRESET_CLICKS[preset_name] + base_clicks,
                )
                preset_reviews[preset_name] = preset_out / "index.html"
            review = write_preset_index(preset_reviews, args.out)
            print(f"preset review: {review}")
        else:
            if args.preset and not view_args:
                view_args = preset_default_views(args.preset)
            views = selected_views(view_args, args.quick, available_views)
            clicks = base_clicks
            if args.preset:
                clicks = PRESET_CLICKS[args.preset] + clicks
            run_audit(views, sizes, args.out, binary, clicks)
    except Exception as error:
        print(f"ui layout audit failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

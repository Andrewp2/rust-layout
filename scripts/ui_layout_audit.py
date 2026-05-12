#!/usr/bin/env python3
from __future__ import annotations

import argparse
import html
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


def capture_snapshot(binary: Path, view: ViewCase, size: ViewportSize, out_dir: Path) -> Path:
    raw_path = out_dir / "raw" / size.name / f"{view.slug}.rgba"
    png_path = out_dir / "screenshots" / size.name / f"{view.slug}.png"
    summary_path = out_dir / "summaries" / size.name / f"{view.slug}.txt"
    raw_path.parent.mkdir(parents=True, exist_ok=True)
    png_path.parent.mkdir(parents=True, exist_ok=True)
    summary_path.parent.mkdir(parents=True, exist_ok=True)
    command = [
        str(binary),
        "--operad-snapshot",
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
<p>Review each Operad snapshot for clipped labels, bad wrapping, crowded margins, and content that disappears at narrow or wide aspect ratios.</p>
<h2>Contact sheets</h2>
<ul>{sheet_links}</ul>
{"".join(rows)}
"""
    path = out_dir / "index.html"
    path.write_text(html_text, encoding="utf-8")
    return path


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
    binary = native_binary()
    screenshots: dict[tuple[str, str], Path] = {}

    for size in sizes:
        for view in views:
            screenshots[(size.name, view.slug)] = capture_snapshot(binary, view, size, out_dir)
            print(f"captured {size.name}/{view.slug}")

    contact_sheets = [make_contact_sheet(size, views, screenshots, out_dir) for size in sizes]
    review = write_review_html(sizes, views, screenshots, contact_sheets, out_dir)
    print(f"review: {review}")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Capture Fabricad Operad UI snapshots across views and aspect ratios."
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

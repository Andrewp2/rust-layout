# Hosting Plan

This repository is ready for an internet-hosted browser demo through its existing
WebAssembly path. `crates/wasm_app` exposes the shared validation surface
through wasm-bindgen, and `Trunk.toml` builds `crates/wasm_app/index.html` into
`dist/`.

Recommended first deployment: publish the Trunk `dist/` output as a static site on
GitHub Pages with GitHub Actions. This is the smallest operational surface for the
current architecture: no application server is needed for the editor itself, the
build stays in the repository CI, and the app can be served as static HTML, JS, and
WASM.

## Current Architecture

- `native_app` owns the native window, audit, and snapshot entry points,
  and it has a `web` feature used by `wasm_app`.
- `wasm_app` is browser-capable today. A release build succeeds with:

  ```bash
  env -u NO_COLOR trunk build --release
  ```

- The verified release output is `dist/` with `index.html`, one JS loader, and one
  WASM binary. The local build produced a roughly 13 MB `dist/` directory.
- `sync_server` is optional collaboration infrastructure. Static hosting is enough
  for a single-user browser demo, but collaboration requires a separately hosted
  WebSocket service.

## Recommended Path: GitHub Pages

Use GitHub Pages once this local repository is pushed to GitHub.

Required repository changes:

1. Add a GitHub Actions workflow, for example `.github/workflows/deploy-web.yml`.
2. In GitHub repository settings, set Pages source to "GitHub Actions".
3. Keep `dist/` ignored. CI should upload it as a build artifact, not commit it.
4. For a project page at `https://<owner>.github.io/<project-slug>/`, build with
   `--public-url /<project-slug>/`. For a custom domain or user/org page at the
   domain root, use `--public-url /`.
5. Unset `NO_COLOR` in the workflow because this Trunk version rejects `NO_COLOR=1`.

Workflow shape:

```yaml
name: Deploy web

on:
  push:
    branches: [main]
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: false

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - name: Install Trunk
        run: cargo install trunk --locked
      - name: Check core model
        run: cargo test -p layout_model
      - name: Build web app
        run: env -u NO_COLOR trunk build --release --public-url /<project-slug>/
      - uses: actions/upload-pages-artifact@v4
        with:
          path: dist

  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@v4
```

Add `cargo check --workspace` to a separate CI job or before the deploy build if
deploy time is acceptable. Keep `scripts/render_e2e.py` as a post-deploy or nightly
browser smoke test rather than blocking every deploy until the web rendering surface
is stable enough.

## Cloudflare Pages Alternative

Use Cloudflare Pages if the priority is a custom domain, branch preview URLs, or
Cloudflare-managed CDN controls. The simplest reliable approach is still to build in
GitHub Actions and upload the prebuilt `dist/` directory with Wrangler:

```bash
env -u NO_COLOR trunk build --release --public-url /
npx wrangler pages deploy dist --project-name <pages-project>
```

Required setup:

- Create a Cloudflare Pages project.
- Add `CLOUDFLARE_ACCOUNT_ID` and `CLOUDFLARE_API_TOKEN` as GitHub Actions secrets.
- Use `cloudflare/wrangler-action` or `npx wrangler pages deploy dist`.

This avoids depending on the hosted Pages build image having the exact Rust, wasm
target, and Trunk setup required by the repo.

## Netlify Alternative

Netlify can host the same `dist/` output. Use it if its deploy previews or team
workflow are already preferred.

Build settings:

- Build command: `rustup target add wasm32-unknown-unknown && cargo install trunk --locked && env -u NO_COLOR trunk build --release --public-url /`
- Publish directory: `dist`

If install time is too slow or brittle, build in GitHub Actions and deploy `dist/`
with the Netlify CLI instead.

## Collaboration Hosting

Static hosting does not host collaboration. The browser client defaults to
`wss://<page-host>:4141/ws` on HTTPS pages, or a URL supplied as `?sync=...`.

For collaboration:

1. Host `sync_server` as a small Rust service on a platform such as Fly.io, Render,
   Railway, a VPS, or a container host.
2. Bind it externally with `GLASSWORKS_SYNC_ADDR=0.0.0.0:4141`, or adapt startup to
   consume a platform-provided `PORT`.
3. Put it behind TLS and expose `wss://sync.<domain>/ws`.
4. Launch the static app with `?sync=wss://sync.<domain>/ws`, or add a small runtime
   config/default URL so users do not need a query string.
5. Set `GLASSWORKS_SYNC_STATE=/data/state.json` if persistent collaboration state is
   needed; otherwise use ephemeral state for demos.

This can be a second phase. The first public demo should ship without collaboration
as a static app.

## Native Streaming Fallback

Do not use VNC, WebRTC desktop streaming, or a remote native GUI as the primary
hosting path. They add server cost, latency, GPU/windowing issues, authentication
work, and operational complexity. Keep that option only if a future feature cannot
work in WebAssembly.

## Risks And Follow-Up Work

- WASM size: the release `wasm` artifact is about 12.9 MB before CDN compression.
  Enable gzip or Brotli on the host and consider re-enabling `wasm-opt` after build
  stability is verified.
- Public URL: GitHub Pages project sites need a subpath-aware Trunk build, for
  example `--public-url /<project-slug>/`.
- Browser rendering: verify Chrome, Firefox, and Safari behavior for WebGL,
  WebGPU, and large layouts.
- Collaboration default URL: the current browser default assumes the sync server is
  on the same hostname at port `4141`, which is not how most static hosts expose TLS.
- Shader build reproducibility: CI can either allow the current best-effort shader
  build hook, install `slangc`, or set `GLASSWORKS_SKIP_SHADER_COMPILE=1` while relying
  on checked-in compiled WGSL assets.
- Repository publishing: this local worktree currently has no git remote configured,
  so hosting starts by creating or connecting a remote repository.

## References Checked

- Trunk is the Rust WASM bundler used by this repo:
  https://github.com/trunk-rs/trunk
- GitHub Pages supports custom GitHub Actions workflows for non-Jekyll builds:
  https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site
- Cloudflare Pages can deploy prebuilt assets through Wrangler:
  https://developers.cloudflare.com/pages/how-to/use-direct-upload-with-continuous-integration/
- Netlify supports Git-connected builds, CLI deploys, and drag-and-drop static
  deploys:
  https://docs.netlify.com/deploy/create-deploys/

# Fabricad Roadmap

Fabricad is a Rust/WASM mask-layout editor for simplified semiconductor fabrication geometry. The project goal is not to clone an industrial EDA suite outright; it is to build a credible CAD/Figma-style editor that demonstrates precise geometry editing, large-layout rendering, spatial indexing, routing, DRC, collaboration, and native/WebAssembly Rust architecture.

This roadmap is the working checklist. Each milestone should leave the app in a demonstrable state with tests or screenshots proving the important behavior.

## Current State

The app already has a usable baseline:

- Native egui/wgpu editor.
- WebAssembly build through Trunk.
- Shared Rust crates for geometry, layout data, DRC, routing, rendering, sync, native, and web.
- Slang shader source compiled to WGSL and SPIR-V.
- Infinite canvas with pan and zoom.
- Bottom-left nm/um scale bar tied to the current zoom level.
- Named process layers with visibility controls.
- JSON technology files for layer definitions, purposes, colors, display order, grid, connectivity, and DRC rules, with native technology switching.
- GDSII import/export for the current Fabricad subset: structures, references, AREF instance arrays, boundaries, paths, text, and layer/datatype mapping.
- Connectivity extraction across the active technology stack, selected-net highlighting, label propagation, and open/short reports.
- Rectangle, polygon, path, via, and measurement drawing.
- Shape selection and drag editing with grid snapping.
- Vertex handles and vertex dragging for selected top-level rectangles, polygons, and paths.
- Vertex insertion/deletion, rectilinear edge dragging, copy/paste/duplicate/delete, rotate/mirror, and a selected-shape property panel.
- Undo and redo for local edits.
- R-tree spatial index for hit testing and range queries.
- Tiled renderer with dirty-tile invalidation and dense-layout LOD summaries.
- GPU fill rendering and GPU picking pass.
- Stress layouts at 10k, 100k, and 1M rectangles.
- Simplified DRC: min width, spacing, enclosure, forbidden overlap, and off-grid checks.
- A* maze routing around blocked geometry.
- Rust WebSocket sync server with operation log and remote cursor messages.
- Loro-backed CRDT sync with actor/counter IDs, dependency frontiers, duplicate-safe semantic application, Loro update bytes, WebSocket protocol support, and Loro maps/registers/tombstones for shapes, cells, and instances.
- Screenshot E2E harness for deterministic render validation.
- Hierarchy scaffolding: cells, instances, translated transforms, flattening APIs, hierarchy-aware R-tree indexing, renderer integration, GPU picking by occurrence, and a screenshot-tested hierarchy scene.
- Basic hierarchy editing: create cell from selection, place instances, select instance occurrences, move instance transforms, rename cells/instances, edit instance translation in the property panel, and synchronize hierarchy operations through the operation log.

## KLayout-Grade Gaps

These are the largest gaps versus mature layout tools such as KLayout. They are ordered by portfolio value for Fabricad rather than by industrial completeness.

1. Hierarchy: cells, instances, arrays, transforms, and a cell tree.
2. Real layout formats: at least GDSII import/export, then OASIS later.
3. More complete editing: boolean operations, richer instance placement, and more CAD-grade transform workflows.
4. Richer technology support: external file loading, multiple foundry-like stacks, layer/datatype mapping, and scriptable rule decks.
5. Net tracing and connectivity extraction across metal/contact/via stacks.
6. Scriptable or data-driven DRC instead of fixed Rust-only rules.
7. Marker browser for DRC errors, measurements, and review annotations.
8. Collaboration hardening: browser-side sync, conflict tests, reconnect/resync behavior, and persisted operation logs.
9. Performance architecture for very large layouts: hierarchy-aware culling, tile streaming, GPU-resident metadata, and out-of-core storage.
10. Automation and scripting APIs.

## Milestone 1: Hierarchical Layout Model

Why this matters: hierarchy is the biggest conceptual difference between a drawing demo and a real mask-layout editor. It also sets up GDS import/export, instancing benchmarks, and realistic large-layout rendering.

Scope:

- Add `CellId`, `Cell`, `InstanceId`, `CellInstance`, and `Transform` to `layout_model`.
- Represent a document as a set of cells with one top cell.
- Support top-level shapes and instance references.
- Flatten visible geometry for the current renderer without changing the renderer API too much.
- Update the R-tree to index flattened instance geometry.
- Keep operation-log compatibility for existing shape edits where possible.
- Add JSON round-trip tests for documents containing cells and instances.

Acceptance checks:

- A document can contain at least two cells.
- The top cell can instantiate another cell multiple times with translation.
- Selection and rendering show instantiated shapes in the right positions.
- Save/load preserves hierarchy.
- `cargo test --workspace` passes.
- Screenshot E2E includes a hierarchy scene with repeated instances.

First implementation pass:

1. Add hierarchy data types to `layout_model`.
2. Add helpers for default top-cell construction.
3. Add flattening APIs that return shape references plus composed transforms.
4. Update renderer/index call sites to use flattening.
5. Add a generated demo scene with repeated standard-cell-like blocks.

## Milestone 2: Better Editing Tools

Why this matters: low-latency, precise editing is the core product surface. The current app can draw and move shapes, but mature CAD workflows need direct manipulation of vertices and edges.

Scope:

- Vertex selection handles for polygons and paths.
- Drag individual vertices with grid snapping.
- Insert and delete vertices.
- Edge drag for rectilinear polygons.
- Rotate and mirror selected objects.
- Copy, paste, duplicate, and delete commands.
- Property panel for selected shape layer, label, net, and coordinates.

Acceptance checks:

- Vertex edits are represented as operations.
- Undo/redo works for vertex, edge, transform, copy, and delete edits.
- Remote clients converge when vertex edits and deletes are interleaved.
- Screenshot E2E covers selected shape handles and vertex movement.

Initial slice completed:

- Selected top-level rectangles, polygons, and paths show screen-space vertex handles.
- Dragging a handle snaps to the grid and applies an undoable `ReplaceShape` operation.
- Instance geometry is not edited through occurrences; instance dragging still moves the instance transform.
- Added tests for polygon and rectangle vertex replacement helpers.
- Added a deterministic screenshot E2E case for selected vertex handles.

Completion pass:

- Added double-click edge insertion for polygons, paths, and rectangles. Rectangle insertion converts the rectangle to a polygon so further vertex editing is explicit.
- Added hover-delete for polygon/path vertices, with minimum vertex-count guards.
- Added rectilinear edge dragging for rectangles and Manhattan polygon edges.
- Added copy, paste, duplicate, and delete commands for top-level shapes, plus delete for selected instances.
- Added rotate-90, mirror-X, and mirror-Y commands for selected top-level shapes.
- Added a selected-shape property panel for layer, name, net, coordinates, path width, label text, and measurement endpoints.
- Kept the new edit flows undoable through `ReplaceShape`, `AddShape`, `DeleteShape`, and `Batch` operations.
- Tightened delete-vs-vertex-edit semantics so `ReplaceShape` does not resurrect a deleted shape.
- Added helper/model tests and a screenshot E2E case for a moved vertex.

## Milestone 3: Technology Files And Rule Decks

Why this matters: semiconductor CAD tools are driven by process definitions. A fake but data-driven process stack will make Fabricad feel much closer to the target domain.

Scope:

- Add a technology file format, likely `toml` or `json`.
- Define layers, colors, purposes, DBU, snap grid, and display order.
- Define connectivity: metal/contact/via stacks.
- Define DRC rules as data.
- Load the default technology at startup.
- Allow switching technologies from the native app.

Acceptance checks:

- The default demo is loaded from a technology file.
- DRC rule thresholds are not hard-coded in the UI.
- A test technology can change min spacing and produce different violations.
- Invalid technology files produce useful errors.

Completion pass:

- Added built-in JSON technology files under `assets/technology/`.
- Added a `TechnologyFile` schema with layer definitions, purposes, display order, grid, DBU scale, connectivity, and DRC rule data.
- `Document::new` and demo/stress/hierarchy scenes now derive their layers from the default technology file.
- `RuleDeck::from_technology` builds min-width, min-spacing, enclosure, and forbidden-overlap rules from data.
- Added a native side-panel technology picker that reapplies layer definitions and rebuilds DRC rules.
- Added tests for default technology loading, invalid technology validation, and DRC behavior changing when technology spacing rules change.

## Milestone 4: GDSII Import And Export

Why this matters: format support makes the app interoperable and demonstrates schema translation, integer coordinate handling, and hierarchy.

Scope:

- Start with GDSII export for Fabricad documents.
- Support boundary polygons, paths, text labels, structures, references, and arrays.
- Add GDSII import after export is stable.
- Preserve layer/datatype mappings through the technology file.
- Add round-trip tests for small hierarchy fixtures.

Acceptance checks:

- Exported files open in KLayout with expected layers and hierarchy.
- Simple KLayout-created fixtures import into Fabricad.
- Round-trip tests preserve geometry within DBU precision.

Completion pass:

- Added technology-level GDS layer/datatype/texttype mapping and duplicate mapping validation.
- Added a shared binary GDSII reader/writer in `layout_model`.
- Export maps Fabricad cells to GDS structures, instances to SREFs, rectangles/polygons/vias to BOUNDARY records, paths to PATH records, and labels to TEXT records.
- Import maps structures back into Fabricad cells, references into instances, boundaries/paths/text into Fabricad shapes, and unknown GDS layers into generated document layers.
- Imported AREF arrays are preserved as first-class Fabricad instance arrays.
- Added native toolbar buttons for fixed-path GDS export/import at `examples/fabricad_layout.gds`.
- Added tests for GDS real8 units, hierarchy round-trip, AREF fixture import/export, unknown layer import, and technology mapping validation.

## Milestone 5: Net Tracing And Connectivity

Why this matters: routing and layout verification become much more meaningful once the editor understands connected geometry.

Scope:

- Define layer-to-layer connectivity through contacts and vias.
- Build a graph of connected shapes.
- Select a shape and highlight its connected net.
- Add net labels and net-name propagation.
- Report open routes and shorted labels.

Acceptance checks:

- Clicking a metal shape highlights the connected metal/via stack.
- Two labels on the same connected component agree or report a short.
- Router output is assigned to the expected net.

Completion pass:

- Added a shared connectivity extractor in `layout_model`.
- Connectivity uses the technology file's `connectivity` stack to join same-layer overlaps and layer-to-layer contact/via overlaps.
- Flattened hierarchy occurrences participate in net extraction, so repeated instances can be traced independently.
- Label shapes propagate net names to the component under their label point.
- Components report explicit numeric net IDs, inferred net names, shorts from conflicting labels/net IDs, and opens from repeated labels on disconnected islands.
- Native side panel now shows connectivity timing, selected net details, shorts, and opens.
- Selecting a shape highlights the connected component on the canvas.
- Router output inherits a matching endpoint net ID or label name when one can be inferred.
- Via placement now defaults to the technology's via1 layer while retaining lower/upper metal links.
- Added tests for via-stack propagation, conflicting labels, open labels, and explicit net IDs.

## Milestone 6: Collaboration Hardening

Why this matters: live collaboration is a key differentiator for the project, but it needs deterministic tests and browser integration to be convincing.

Current direction:

- Treat user edits as semantic commands, not raw patches.
- Wrap commands in CRDT operation envelopes with stable actor/counter IDs, then store and replicate them through Loro updates.
- Use Loro for replicated update exchange and object maps now, then add richer geometry-level merge semantics incrementally.
- Model undo/redo as new semantic inverse commands, not history rewrites.

Scope:

- Finish Loro-backed client behavior for native and WASM.
- Use Loro object-map materialization for reconnect and persisted startup recovery.
- Add WebSocket client support for the WASM app.
- Persist server Loro snapshots/update logs to disk.
- Add reconnect and snapshot recovery.
- Add conflict tests for move/delete, vertex/delete, concurrent layer changes, and duplicate inserts.
- Show remote selections and remote cursors in both native and web.

Acceptance checks:

- Two native clients and two browser clients can edit the same document.
- A disconnected client can reconnect and converge.
- Conflict behavior is documented and tested.

Completion pass:

- Added persisted sync-server state at `FABRICAD_SYNC_STATE`, defaulting to `target/fabricad-sync/state.json`.
- Server startup now imports the persisted Loro snapshot and materializes shape/cell/instance object maps back into the authoritative document.
- Native clients apply collaboration snapshots through the same Loro materialization path used by persisted recovery.
- Added a WebSocket collaboration client for WASM builds, with URL discovery from `?sync=...` or `ws://<page-host>:4141/ws`.
- Added shared remote selection protocol messages and rendered remote selections alongside remote cursors.
- Broadcast selection presence from the editor without spamming unchanged selections.
- Added conflict tests for move/delete, vertex/delete, duplicate inserts, and ordered concurrent layer visibility changes.
- Added a sync-server recovery test proving persisted startup uses the Loro object store rather than only the fallback JSON document.
- Added a four-client WebSocket smoke test covering Loro edit broadcast, selection/cursor presence, disconnect, reconnect, and snapshot convergence, plus JSON ID-key round-trip coverage for collaboration snapshots.

## Milestone 7: Larger Layout Rendering

Why this matters: the app should have a clear performance story beyond drawing many random rectangles.

Scope:

- Hierarchy-aware tile culling.
- Per-tile GPU buffers or indirect draw batches.
- Tile streaming and eviction.
- Pick acceleration that avoids full-scene pick batches.
- GPU memory budget reporting.
- Benchmarks for repeated hierarchy instances and dense flat polygons.

Acceptance checks:

- Stress scenes include flat geometry and hierarchical repeated cells.
- Performance overlay reports visible tiles, resident tiles, GPU buffer bytes, CPU build time, pick time, and query time.
- Screenshot E2E verifies LOD and non-LOD views.

Completion pass:

- Tile cache now tracks resident tile count, resident shape-batch count, evicted tiles, evicted shape batches, memory budget, and over-budget bytes.
- Added an LRU-style memory-budget pass that evicts old non-visible tiles first and then prunes orphaned shape batches.
- Renderer frames now include indirect-draw-style ranges for LOD tile overviews and precise shape chunks.
- Native performance overlay now reports resident/evicted tiles, resident/evicted shape batches, draw range count, pick candidate count, pick-build time, and tile-cache budget pressure.
- Existing tiled rendering continues to use hierarchy-aware R-tree occurrence queries and dense-layout LOD overview tiles.
- Added renderer tests for cache eviction under a tight budget, draw range emission, and pick batches being limited to visible tile occurrences instead of the full scene.
- Existing render E2E scenes cover hierarchy, precise selected-handle rendering, moved-vertex rendering, and stress LOD rendering.

## Milestone 8: Scriptable DRC And Marker Browser

Why this matters: DRC should evolve from a fixed demo feature into a small verification system.

Scope:

- Add a marker browser panel for DRC violations.
- Support waiving or hiding selected markers.
- Make DRC incremental by dirty region where practical.
- Add data-driven checks for width, spacing, enclosure, overlap, and off-grid.
- Consider a small scripting layer only after data-driven rules are solid.

Acceptance checks:

- Clicking a marker focuses and selects the relevant geometry.
- Marker state survives save/load.
- Rule changes are reflected without recompiling.

Completion pass:

- Added persisted `MarkerState` entries to `Document`, keyed by stable DRC marker signatures and defaulting cleanly for old documents.
- Native side panel now has a DRC marker browser with active/total counts, waived/hidden filters, per-marker Waive/Unwaive and Hide/Show controls, and a Clear action.
- Clicking a marker focuses the marker bounds and selects the referenced top-level shapes.
- Hidden and waived markers are suppressed from canvas violation rendering and active marker counts.
- Marker hide/waive state survives the existing JSON save/load path.
- Added a conservative dirty-region DRC merge path that preserves previous markers outside affected bounds and refreshes markers intersecting the dirty region.
- DRC rules remain data-driven through technology files, so rule changes do not require recompiling.
- Added tests for marker-state JSON round-trip, stable marker keys, and dirty-region marker preservation.

## Milestone 1 Progress

Completed first slice:

- Added hierarchy structs and serialization in `crates/layout_model`.
- Kept existing flat-document behavior working through top-level `Document.shapes`.
- Added tests for default documents, save/load, old flat JSON compatibility, translated instance flattening, and hierarchy-aware indexing.
- Left existing flat edit operations unchanged.

Completed second slice:

- Added renderer-facing occurrence IDs so repeated instances do not collide on the source `ShapeId`.
- Updated `TileCache`, GPU picking, CPU fallback rendering, and native selection hit testing to consume flattened hierarchy.
- Added a generated hierarchy demo scene with repeated standard-cell-like blocks.
- Added screenshot E2E coverage and a baseline for `?scene=hierarchy`.

Completed third slice:

- Added operation-log variants for batches, cells, instances, instance deletion, and instance movement.
- Added `Make Cell` for factoring selected top-level shapes into a normalized child cell and replacing them with a top-cell instance.
- Added a side-panel cell list with `Place` actions for reusable cells.
- Added instance occurrence selection and transform dragging.
- Added model tests for cell/instance add, move, delete, and batch operation behavior.

Completed polish pass:

- Added rename operations for cells and instances.
- Added editable cell names in the hierarchy side panel.
- Added a selected-instance property panel with name and X/Y translation controls.
- Added model tests for rename operations.
- Refreshed screenshot baselines for the hierarchy UI.

Completed hierarchy polish pass:

- Added first-class `InstanceArray` metadata with row/column counts and independent row/column pitch vectors.
- Extended instance `Transform` from translation-only to orthogonal matrix plus translation, with rotate-90, mirror-X, and mirror-Y editing support.
- Flattened arrayed instances into distinct occurrence IDs so rendering, picking, selection, and tile-cache entries do not collide.
- Added selected-instance side-panel controls for rotation, mirroring, array dimensions, and array pitch.
- Preserved GDSII AREF arrays as Fabricad instance arrays and export arrayed instances back as AREF records.
- Changed cell and instance name fields to keep local drafts and commit renames only on Enter or focus loss, avoiding per-keystroke undo entries.
- Added a deterministic screenshot E2E case for the create-cell/place-instance workflow.
- Added model, Loro object-store, old-document compatibility, GDS import, GDS export/import, and flattening tests for arrays and oriented transforms.

Started real 3D viewport renderer:

- Added renderer-owned 3D vertex batches and an egui-wgpu callback path that renders the 3D view into a dedicated offscreen color texture with a depth attachment.
- Added a wgpu 3D scene pipeline with depth test/write for opaque layout solids, followed by a separate composite pass back into the egui UI.
- Kept HUD and future handles as egui overlays above the composited viewport, with the old painter-sorted 3D path isolated as a no-wgpu fallback.
- Added focused invariants for 3D batch fingerprinting, face triangulation, and WGSL matrix upload ordering.

## Immediate Next Checkpoint

The roadmap milestones and listed hierarchy polish items are implemented. The next task should be a completion audit unless one of the follow-on polish areas takes priority.

Concrete options:

- Completion audit: verify each milestone against code, docs, tests, screenshots, and remaining known gaps.
- Collaboration smoke test: run two native clients and two `trunk serve` browser clients against one sync server, edit from each, disconnect/reconnect one client, and confirm convergence.
- Connectivity polish: let routing prefer known net components and add a net-label drawing tool.
- Format polish: add a tiny checked-in KLayout-created GDS fixture if available.
- Verification improvement: make DRC and routing consume flattened hierarchy instead of only top-level shapes.
- Hierarchy polish: add richer orientation-aware GDS STRANS/ANGLE import/export instead of preserving orientation only inside Fabricad.

Suggested command loop:

```bash
cargo fmt --all -- --check
cargo test -p layout_model
cargo test --workspace
./scripts/render_e2e.py --skip-baseline
```

## Definition Of Done For Each Slice

Every implementation slice should end with:

- A visible behavior change or a tested internal capability.
- Focused unit tests for model, geometry, DRC, routing, or sync logic where relevant.
- Screenshot E2E coverage for rendering or UI changes.
- README or roadmap updates when the user-facing workflow changes.
- No known panic in `cargo run --bin fabricad`.

## Non-Goals For Now

These are real EDA concerns, but they should not distract the near-term build:

- Full foundry-accurate process design kits.
- Full OASIS support before GDSII basics.
- Full CRDT editing semantics before operation-order sync is proven.
- Transistor-level LVS before connectivity extraction exists.
- Hand-tuned billion-triangle rendering before hierarchy and tile streaming are in place.

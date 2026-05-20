# Glassworks Roadmap

Glassworks is a Rust/WASM mask-layout editor for simplified semiconductor fabrication geometry. The project goal is not to clone an industrial EDA suite outright; it is to build a credible CAD/Figma-style editor that demonstrates precise geometry editing, large-layout rendering, spatial indexing, routing, DRC, collaboration, and native/WebAssembly Rust architecture.

This roadmap is the working checklist. Each milestone should leave the app in a demonstrable state with tests or screenshots proving the important behavior.

## Current State

The app already has a usable baseline:

- Native winit/wgpu window, audit path, and snapshot path.
- WebAssembly build through Trunk.
- Shared Rust crates for geometry, layout data, DRC, routing, rendering, sync, native, and web.
- Slang shader source compiled to WGSL and SPIR-V.
- Infinite canvas with pan and zoom.
- Bottom-left nm/um scale bar tied to the current zoom level.
- Named process layers with visibility controls.
- JSON technology files for layer definitions, purposes, colors, display order, grid, connectivity, and DRC rules, with native built-in technology switching that drives connectivity, GDS mapping, and built-in DRC plus per-link connectivity enable controls for net extraction.
- GDSII import/export for the current Glassworks subset: structures, references, AREF instance arrays, boundaries, paths, text, and layer/datatype mapping.
- Native File-menu layout JSON and workspace JSON save/load through validated model serializers.
- Native File-menu deterministic app-session save/load for workspace data, app options, shell state, and the current 2D layout view.
- Persisted reference-image overlay metadata with deterministic JSON exchange, visibility control, and landmark-based axis-aligned alignment.
- Connectivity extraction across the active technology stack, selected-net highlighting, label propagation, and open/short reports.
- Rectangle, polygon, path, via, label, and measurement drawing.
- Shape selection and drag editing with grid snapping.
- Vertex handles and vertex dragging for selected top-level rectangles, polygons, and paths.
- Vertex insertion/deletion, rectilinear edge dragging, copy/paste/duplicate/delete, rotate/mirror, and a selected-shape property panel.
- Undo and redo for local edits.
- R-tree spatial index for hit testing and range queries.
- Tiled renderer with dirty-tile invalidation and dense-layout LOD summaries.
- GPU fill rendering and GPU picking pass.
- Stress layouts at 10k, 100k, and 1M rectangles.
- Simplified DRC: min width, max width, min area, max area, bounds spacing, axis-aligned edge spacing, enclosure, forbidden overlap, off-grid checks, and custom-deck bounds-based named `AND`/`OR`/`NOT` derived layers with derived min/max-width, min/max-area, spacing, edge-spacing, and forbidden-overlap checks.
- A* maze routing around blocked geometry.
- Rust WebSocket sync server with operation log and remote cursor messages.
- Loro-backed CRDT sync with actor/counter IDs, dependency frontiers, duplicate-safe semantic application, Loro update bytes, WebSocket protocol support, and Loro maps/registers/tombstones for shapes, cells, and instances.
- Screenshot E2E harness for deterministic render validation.
- Hierarchy scaffolding: cells, instances, translated transforms, flattening APIs, hierarchy-aware R-tree indexing, renderer integration, GPU picking by occurrence, and a screenshot-tested hierarchy scene.
- Basic hierarchy editing: create cell from selection, place instances, select instance occurrences, make shallow cell variants, duplicate the current view cell, move instance transforms, move local shapes and instances up into parent placements, rename cells/instances, flatten selected instances or the current view cell one level by promoting nested instances or deeply into local geometry, set the current cell origin from selected geometry or grid-derived X/Y offsets, edit instance translation in the property panel, and synchronize hierarchy operations through the operation log.
- Small library macro surface: configure, generate, and place a reusable static via-array cell from the 2D cell browser.

## KLayout-Grade Gaps

These are the largest gaps versus mature layout tools such as KLayout. They are ordered by portfolio value for Glassworks rather than by industrial completeness.

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

- Selected current-cell rectangles, polygons, and paths show screen-space vertex handles.
- Dragging a handle snaps to the grid and applies an undoable `ReplaceShape` operation.
- Instance geometry is not edited through occurrences; instance dragging still moves the instance transform.
- Added tests for polygon and rectangle vertex replacement helpers.
- Added a deterministic screenshot E2E case for selected vertex handles.

Completion pass:

- Added double-click edge insertion for polygons, paths, and rectangles. Rectangle insertion converts the rectangle to a polygon so further vertex editing is explicit.
- Added hover-delete for polygon/path vertices, with minimum vertex-count guards.
- Added rectilinear edge dragging for rectangles and Manhattan polygon edges.
- Added copy, paste, duplicate, and delete commands for selected current-cell shapes, plus delete for selected instances.
- Added rotate-90, mirror-X, and mirror-Y commands for selected current-cell shapes and selected instances.
- Added a selected-shape property panel for layer, name, net, coordinates, path width, label text, and measurement endpoints.
- Kept the new edit flows undoable through `ReplaceShape`, `AddShape`, `DeleteShape`, and `Batch` operations.
- Tightened delete-vs-vertex-edit semantics so `ReplaceShape` does not resurrect a deleted shape.
- Added helper/model tests and a screenshot E2E case for a moved vertex.

## Milestone 3: Technology Files And Rule Decks

Why this matters: semiconductor CAD tools are driven by process definitions. A fake but data-driven process stack will make Glassworks feel much closer to the target domain.

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
- `RuleDeck::from_technology` builds min-width, max-width, min-area, max-area, min-spacing, enclosure, and forbidden-overlap rules from data.
- Added a native side-panel technology picker that reapplies layer definitions and rebuilds DRC rules.
- Added tests for default technology loading, invalid technology validation, and DRC behavior changing when technology spacing rules change.

Format broadening pass:

- Added a CIF subset reader/writer in `layout_model` for boxes, polygons, wires, labels, vias-as-boxes, and basic hierarchical `DS`/translated-`C` import/export.
- Added File-menu CIF export/import using a deterministic `target/glassworks-layout.cif` exchange path in the native app, including gzip- and single-file-ZIP-aware CIF import and recent-file reload support.
- Added focused model and native-app coverage for CIF round-tripping, generated unknown layers, basic hierarchy calls, File-menu CIF exchange, and compressed CIF import.
- Added an ASCII DXF subset reader/writer in `layout_model` for closed/open polylines, lines, text labels, circles/arcs, basic rotated/mirrored `BLOCK`/`INSERT` hierarchy, generated unknown layers, and skipped unsupported entities.
- Added File-menu DXF export/import using a deterministic `target/glassworks-layout.dxf` exchange path in the native app, including gzip- and single-file-ZIP-aware DXF import and recent-file reload support.
- Added focused model and native-app coverage for DXF round-tripping, old POLYLINE/VERTEX import, circle/arc import, basic rotated/mirrored block/insert hierarchy, unsupported-entity reporting, File-menu DXF exchange, and compressed DXF import.
- Added a DEF subset reader/writer in `layout_model` for `FILLS` rectangles/polygons, `SPECIALNETS`/`NETS` routed paths, placed `PINS` labels, basic orthogonal `COMPONENTS` placeholder instances, generated unknown layers, and skipped bad items.
- Added File-menu DEF export/import using a deterministic `target/glassworks-layout.def` exchange path in the native app, including gzip- and single-file-ZIP-aware DEF import and recent-file reload support.
- Added focused model and native-app coverage for DEF round-tripping, unknown-layer generation, route repeat-coordinate parsing, placed pin labels, basic oriented component placement metadata, File-menu DEF exchange, and compressed DEF import.
- Added a basic multi-macro LEF macro-cell subset reader/writer in `layout_model` for `OBS` rectangles/polygons, Manhattan paths as obstruction rectangles, `PIN PORT` labels, generated unknown layers, and skipped bad items.
- Added File-menu LEF export/import using a deterministic `target/glassworks-layout.lef` exchange path in the native app, including gzip- and single-file-ZIP-aware LEF import and recent-file reload support.
- Added focused model and native-app coverage for LEF round-tripping, unknown-layer generation, pin-port labels, multi-macro cell preservation, File-menu LEF exchange, and compressed LEF import.

## Milestone 4: GDSII Import And Export

Why this matters: format support makes the app interoperable and demonstrates schema translation, integer coordinate handling, and hierarchy.

Scope:

- Start with GDSII export for Glassworks documents.
- Support boundary polygons, paths, text labels, structures, references, and arrays.
- Add GDSII import after export is stable.
- Preserve layer/datatype mappings through the technology file.
- Add round-trip tests for small hierarchy fixtures.

Acceptance checks:

- Exported files open in KLayout with expected layers and hierarchy.
- Simple KLayout-created fixtures import into Glassworks.
- Round-trip tests preserve geometry within DBU precision.

Completion pass:

- Added technology-level GDS layer/datatype/texttype mapping and duplicate mapping validation.
- Added a shared binary GDSII reader/writer in `layout_model`.
- Export maps Glassworks cells to GDS structures, instances to SREFs, rectangles/polygons/vias to BOUNDARY records, paths to PATH records, and labels to TEXT records.
- Import maps structures back into Glassworks cells, references into instances, boundaries/BOXes/paths/text into Glassworks shapes, and unknown GDS layers into generated document layers.
- Imported AREF arrays are preserved as first-class Glassworks instance arrays, orthogonal SREF/AREF rotations/reflections round-trip through GDS STRANS/ANGLE records, GDS PATH square/custom end extensions import into equivalent Glassworks path endpoints, and Glassworks shape names, net IDs, stored shape key/value metadata, and stored instance key/value metadata round-trip through GDS property records.
- Added File-menu GDS export/import plus GDS import-as-cell, extra-top-cell import, and flattened merge using a deterministic `target/glassworks-layout.gds` exchange path in the native app; the same import-as-cell, extra-top-cell, and merge workflow is also available for CIF, DXF, and DEF exchange.
- Added tests for GDS real8 units, hierarchy round-trip, AREF fixture import/export, SREF/AREF rotation/reflection import/export, PATH end-extension import, shape-name/net-ID/key-value property import/export, instance key-value property import/export, BOX import, unknown layer import, technology mapping validation, and native File-menu GDS exchange.

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

- Added persisted sync-server state at `GLASSWORKS_SYNC_STATE`, defaulting to `target/glassworks-sync/state.json`.
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
- Snapshot render E2E scenes cover workflow, layout, hierarchy, 3D, process-flow, and stress LOD rendering.

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
- Native side panel now has a DRC marker browser with rule-category grouping, active/total counts, waived/hidden/noted filters, per-marker Waive/Unwaive, Hide/Show, review-note controls, and a Clear action.
- Clicking a marker focuses the marker bounds and selects the referenced top-level shapes.
- Hidden and waived markers are suppressed from canvas violation rendering and active marker counts.
- Marker hide/waive state survives the existing JSON save/load path.
- Added a conservative dirty-region DRC merge path that preserves previous markers outside affected bounds and refreshes markers intersecting the dirty region.
- DRC rules remain data-driven through technology files, so rule changes do not require recompiling.
- Added tests for marker-state JSON round-trip, stable marker keys, dirty-region marker preservation, native DRC marker-browser select/filter/state actions, and rule-category filtering.
- Added deterministic DRC deck JSON export/import for an editable layer-name-based rule deck, including max-width, min-area, max-area, axis-aligned min-edge-spacing, and bounds-based named `AND`/`OR`/`NOT` derived-layer min/max-width, min/max-area, min-spacing, min-edge-spacing, plus physical-layer forbidden-overlap thresholds, with imported custom decks overriding future full/cell/region DRC runs and surviving app-session save/load.

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
- Added `Make Cell` for factoring selected current-cell shapes into a normalized child cell and replacing them with a local instance.
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
- Preserved GDSII AREF arrays as Glassworks instance arrays and export arrayed instances back as AREF records.
- Changed cell and instance name fields to keep local drafts and commit renames only on Enter or focus loss, avoiding per-keystroke undo entries.
- Added a deterministic screenshot E2E case for the create-cell/place-instance workflow.
- Added model, Loro object-store, old-document compatibility, GDS import, GDS export/import, and flattening tests for arrays and oriented transforms.

Started real 3D viewport renderer:

- Added renderer-owned 3D vertex batches that render into a dedicated offscreen color texture with a depth attachment.
- Added a wgpu 3D scene pipeline with depth test/write for opaque layout solids.
- Added focused invariants for 3D batch fingerprinting, face triangulation, and WGSL matrix upload ordering.

Closed the first KLayout 2D hierarchy-display gap:

- Added 2D hierarchy display-depth controls for Top, numeric min/max depth ranges, box-only child instances, and Full hierarchy, plus shallower/deeper step controls.
- Added root-cell-aware 2D view indexing, rendering, picking, quick top-cell controls, a flat all-cell browser with current/used/unused/library/property-bearing/empty filters and sort controls, parent/child context actions, and selected-instance descend for focused cell inspection without changing the document top cell.
- Added hidden/visible, leaf, branch, parent, child, sibling, ancestor, and descendant context filters to the cell browser for faster hierarchy navigation.
- Added hierarchy-depth rows, parent/child/sibling/ancestor/descendant relation labels, search fields, and sorting to the cell browser plus shared-search `Cells` inspector summaries so hierarchy lists can be inspected and ordered from shallow used cells through deeper descendants, with unused cells after reachable cells.
- Exposed both one-level and deep selected-instance flatten actions in the hierarchy context controls, matching the Edit menu hierarchy operations.
- Exposed selected array resolving in the hierarchy context controls, matching the Edit menu selected-array operation for document-top, current-cell, and deeper occurrence arrays.
- Added Edit-menu and hierarchy-context current-cell, descendant, and document-wide child-instance batch variants that retarget each child placement to its own shallow-copied cell.
- Added Edit-menu and hierarchy-context descendant-cell and whole-document batch flattening for one-level or deep policies.
- Added Edit-menu and hierarchy-context current-cell, descendant-cell, and whole-document batch array resolving that replaces matching array instances with individual single instances and remains undoable.
- Exposed one-level flatten plus deep/complete cell deletion policies directly in the Edit menu, matching the hierarchy context actions.
- Added a selector-searchable collapsible hierarchy tree in the 2D inspector for navigating parent/child cell paths, with row-count titles, search-revealed matching descendant paths, filtered inspector summaries, empty-search feedback, and bulk expand/collapse controls for the visible hierarchy browser.
- Added view-only individual, sibling, immediate child-cell, and full descendant-cell hide/show from the hierarchy controls; hidden cells are filtered from 2D rendering, picking, and shape browsing without changing DRC/routing/connectivity scope.
- Kept DRC, routing, connectivity, and 3D bounds on full physical hierarchy while limiting only the 2D view index, renderer, and hit-testing path.
- Persisted hierarchy display depth ranges in app options and documented the remaining KLayout cell-management gaps.

Started closing KLayout-style browser gaps:

- Added a 2D shape browser in the layout inspector that lists visible shape occurrences for the current view root/depth and selects flattened occurrences, including instance-backed shapes.
- Added shape-browser kind filters for rectangles, polygons, paths, vias, labels, and measurements.
- Added a 2D instance browser in the layout inspector that lists child instances of the current view root or hierarchy-wide instances owned by reachable child cells, then selects a representative occurrence for the instance.
- Added named-instance, property-bearing-instance, array-instance, hidden-target, visible-target, leaf-target, branch-target, identity-instance, and transformed-instance quick filters for the 2D instance browser.
- Added instance-browser target-cell object rows for target role, local shape count, child-instance count, and local bounds.
- Added live instance-browser filter counts plus active scope/filter/search row-count titles and no-match selection status context.
- Added quick filters for the shape browser, cell browser, instance browser, and net browser, including active-layer, selected-shape, shape-kind, property-bearing shape/cell/instance, empty-cell, selected-net, labeled-net, unlabeled-net, device-connected-net, trace-history-net, SPICE-extra-net, shorted-net, and open-net views.
- Added shared 2D browser text search for cell, shape, instance, extracted-net, and DRC marker browser rows, with native Ctrl/Cmd+F capture.
- Added shared 2D browser column presets for Summary, Geometry, Relations, and All detail rows, including a dedicated cell-browser detail section.
- Added property rows for stored shape metadata, stored instance metadata, stored cell metadata, net, and DRC marker browsers so compact browser labels have inspectable IDs, layers, cells, scopes, bounds, arrays, connectivity, selected-net device-terminal details, and rule/state details.
- Added kind-specific shape-browser object rows for rectangle/polygon area, path point/width/length details, via center/size/stack details, and label text/position.
- Added live cell-browser filter counts plus active filter/search row-count titles and no-match selection status context.
- Added property-selector semantics to the shape, cell, and instance browser Properties filters, where browser search matches only stored property metadata, with shape name/net included for shape selectors, and supports `key=value` searches.
- Added shape-browser selector search for source cells, layers, shape kind/type, name/net, label text, occurrence/bounds metadata, and stored shape properties.
- Added instance-browser selector search for parent and target cells, names, transform/array metadata, target role/count/bounds metadata, and stored instance properties.
- Added cell-browser selector search for cell identity, usage/tree role, hierarchy depth, parent/ancestor/sibling/child/descendant relations, counts, local bounds, and stored cell properties.
- Added sort controls for shape, instance, net, and DRC marker browsers, covering common ID/layer/cell/kind/area/size/rule/state orderings.
- Added live shape-browser filter counts plus active filter/search row-count titles and no-match selection status context.
- Promoted hierarchy depth when instance-browser selection needs child geometry visible for highlight/selection.
- Added selected-shape, selected-instance, and current-cell custom property set/remove controls using the browser search/replace fields, including instance-backed child-cell shapes.
- Added focused native-app tests for shape-browser occurrence selection, selected-shape property controls, instance-browser selection, and browser filter behavior.

Started closing KLayout-style layer display gaps:

- Added layer usage counts across top-level and cell-local shapes.
- Marked layer rows as used or empty in the layer panel and exposed active-layer shape usage in layer properties.
- Added layer-panel visibility and cleanup controls to show all layers, show used layers, hide empty layers, isolate the active layer, invert layer visibility, or prune inactive empty layers with undo support, including live affected-layer counts on visibility presets and removable-layer counts on cleanup.
- Added hierarchy-aware layer deletion from the layer panel, removing and undo-restoring matching top-level and cell-local shapes.
- Added options-backed layer-panel filters for All/FEOL/Routing/Text layer groups, fixed FEOL/Routing child groups, searchable layer group tree summaries with `key=value` selectors for group/parent/current/count metadata, and All/Used/Empty/Visible/Hidden layer rows, with live counts on both group and row filters.
- Added eight named options-backed layer setup slots with layer-panel tabs that expose saved/empty status and visible-layer counts, plus save, restore, per-slot clear, and clear-all controls for visibility, hierarchical layer group filter, layer-row usage filter, and per-layer hierarchy depth overrides.
- Added deterministic layer-set JSON export/import for the saved layer setup slots, including saved setup names.
- Added searchable layer-set inspector summaries with `key=value` selectors for slot/name/state/visibility/group/usage/depth metadata.
- Added per-layer fill and line display style fields, active-layer panel controls, undoable style changes, 2D fill-opacity rendering for non-solid fill styles, hatched, cross-hatched, and normal/dense/sparse stippled rectangular fills, and solid/dashed/dotted/dash-dot outline rendering.
- Added active-layer and current-layer-group hierarchy depth override controls that can cap or expand displayed depth independently of the global hierarchy display, persisted through app options.
- Added focused layout-model and native-app coverage for used/unused layer display, layer visibility presets including active-layer isolation and inversion, saved layer setup slots, layer-set JSON round-trip, layer display style defaults/controls, and per-layer hierarchy depth display filtering.

Started closing KLayout-style net tracing gaps:

- Added a 2D net browser backed by connectivity extraction.
- Net component rows select a representative occurrence and reuse selected-net highlighting.
- Added a point-based Trace tool that selects the extracted net component under a clicked layout point without editing geometry.
- Added a Label tool for placing simple net labels on the active layer, reusing the selected or clicked component name when available.
- Added an explicit Trace All action that extracts all net components, resets the net browser to all/size order, seeds trace history, and selects the largest component for highlighting.
- Added a Trace Path action that checks the first and last route points against extracted connectivity, selects the shared component when connected, and reports disconnected endpoints without editing geometry.
- Added a route-point path preview overlay so Trace Path endpoints remain visually connected even outside the active Route tool.
- Kept pending route-point markers visible for Trace Path previews outside the active Route tool so endpoints remain inspectable while using other layout tools.
- Colored the route-point path preview by extracted connectivity so connected endpoints read as valid and disconnected endpoints read as blocked before clearing the points.
- Updated route-point path preview coloring to evaluate each segment independently for multi-point traces.
- Updated Trace Path execution to validate every adjacent route-point segment, so bad middle segments fail explicitly even when the first and last route points share a net.
- Added failing-segment focus for Trace Path execution so disconnected or untraceable route path segments are zoomed when the action cannot complete.
- Added coordinate-searchable route-point rows and compact route-point summary rows in the net browser so pending trace points can be searched, counted, selected by first match, removed individually, and cross-probed to the actual traceable shape and extracted net under each point.
- Added length- and coordinate-searchable route-point trace path segment rows in the net browser so connected/disconnected/untraceable preview segments can be inspected, filtered with shared browser search, focused by row or first match, and cross-probed into named connected segment nets or disconnected endpoint nets without clearing route points.
- Added `key=value` selector search for route-point and trace-path segment rows, including point/segment indices, endpoint coordinates, statuses, component IDs, and segment lengths.
- Added compact trace path segment summary rows, direct connected-path net selection, path endpoint net selection, first-blocker endpoint net selection, and a focus-blocker control to the net browser so pending route paths show a connected/blocked result, endpoint coordinates/nets, total/listed segment counts, connected/disconnected status, total/connected/blocked path length, and the first blocking segment with coordinates.
- Added net-browser component selector search for component ids, names/net ids, label/device/issue/count metadata, bounds, and optional SPICE status.
- Added trace-history selector search for stored trace positions, latest status, component IDs, and component names so the short history list, History net filter, and trace-state summaries can be filtered by recency as well as net metadata.
- Added a route-point clearing control for clearing pending Trace Path points without dropping trace history.
- Added trace-highlight controls for selected-component, trace-history, and off modes in the net browser.
- Added a selected-trace clearing control in the net browser that clears the highlighted net without dropping trace history.
- Added selected-net source-object, label, connected-device, and short/open issue rows in the net browser so individual shapes, net labels, extracted device objects, and connectivity issue bounds in the highlighted component can be cross-probed without duplicating trace history.
- Added selected-net device-peer rows so other nets connected through the same extracted device can be selected directly from the current net.
- Added selected-net open-peer rows so disconnected components participating in the same open can be walked directly from the net browser.
- Added compact selected-net property summaries for short/open issue details, open peers, and peer device terminals, so net detail rows stay useful before cross-probing into a specific row.
- Extended net browser search to match connected device models, terminals, peer terminals, and short/open issue details, so filtered net views can find the same objects exposed by selected-net rows.
- Added a short in-session trace history list in the 2D inspector so recent traced components can be revisited, filtered with the shared browser search, selected by first matching entry, removed individually, or cleared explicitly.
- Added deterministic extracted-netlist JSON export/import for current connectivity data, reusing the validated connectivity report shape without rerunning extraction on import.
- Added aggregate netlist summary rows to the net browser so imported or extracted connectivity can be checked without opening the JSON export.
- Added recent-file and reload-most-recent integration for extracted-netlist JSON exchange, plus recent-menu labels for netlist, trace-state, and L2N database entries.
- Added global extracted-device rows to the net browser so supported devices can be inspected and cross-probed without selecting one of their connected nets first.
- Added global short/open issue rows to the net browser so extracted connectivity problems can be inspected and cross-probed before selecting an involved net.
- Extended the shared net browser search to filter the global extracted-device and short/open issue rows.
- Added first-matching global netlist item selection for searched extracted-device and connectivity-issue rows.
- Changed connectivity and simple device extraction to use full flattened layout geometry rather than current display visibility, matching DRC/router analysis scope.
- Added deterministic SPICE-style `.subckt` export for current connectivity data, including named nets, generated names for unlabeled components, supported MOS/resistor/capacitor device lines, and comments for opens/shorts.
- Added SPICE `.subckt` schematic comparison for named layout nets and supported extracted MOS/resistor/capacitor device signatures, including exact MOS `L`/`W` dimension checks when the schematic supplies them, with compare status, selected-net SPICE status, missing/extra net summaries, and compact missing/extra device details surfaced in the net browser.
- Added recent-file and reload-most-recent integration for SPICE schematic compare inputs so the last LVS-style comparison can be rerun without reselecting the schematic file.
- Added search-aware SPICE compare issue rows to the net browser for missing/extra nets/devices, with first-matching selection and extra-layout-net/device cross-probing.
- Extended the SPICE Extra net filter to include nets connected through extra layout devices, not only explicitly extra layout nets.
- Added selected-net SPICE device status rows so nets that match schematics by name still expose connected extra-layout-device mismatches and schematic-only missing-device references.
- Extended net browser search to match selected-net SPICE/LVS net and device status, including extra-layout-device and missing-device mismatch text.
- Added missing-device SPICE compare issue cross-probing to jump to the first current layout net referenced by a schematic-only device signature.
- Added explicit missing-net SPICE compare issue status text for schematic-only nets that have no current layout connectivity to cross-probe.
- Added a compact SPICE compare summary section to the net browser panel with status, circuit, net/device counts, mismatch counts, and layout issue count.
- Added a SPICE Issues net-browser filter for nets with any SPICE/LVS net-level or device-level mismatch status.
- Added a SPICE Missing net-browser filter for browsable layout nets implicated by missing-layout net or device compare issues.
- Added a SPICE Nets net-browser filter for layout nets with net-level SPICE/LVS compare mismatches, separating net-name issues from device mismatches.
- Added a SPICE Devices net-browser filter for layout nets implicated by device-level SPICE/LVS compare mismatches.
- Added SPICE issue-net counts to the net browser and inspector compare summaries, matching the SPICE Issues filter predicate.
- Added separate SPICE missing-issue and extra-issue net counts to the net browser and inspector summaries, matching the SPICE Missing and SPICE Extra filters.
- Added separate SPICE net-issue and device-issue net counts to the net browser and inspector summaries, matching the SPICE Nets and SPICE Devices filters.
- Added live count labels to the SPICE Extra, SPICE Nets, SPICE Devices, and SPICE Issues net-browser filter buttons.
- Updated SPICE/LVS net-browser filter status messages to include the same live issue-net counts shown on the filter buttons.
- Made SPICE compare issue action rows honor the active SPICE Extra, SPICE Nets, or SPICE Devices filter while preserving the combined SPICE Issues view.
- Updated the generic Select First Netlist Item action to honor the active SPICE/LVS issue filter for SPICE compare issue rows.
- Added active-row counts to the SPICE Compare Issues section title so filtered LVS-style issue lists show how many rows are visible.
- Added active SPICE/LVS category labels to the SPICE Compare Issues section title for filtered issue lists.
- Clarified SPICE Compare Issues section titles to label their counts as rows, distinct from SPICE filter button net counts.
- Updated SPICE compare issue no-match status messages to include the active SPICE/LVS filter category and browser search text.
- Updated the SPICE Missing issue list to prioritize cross-probeable missing-device rows before schematic-only missing-net rows for select-first actions.
- Updated the SPICE Nets issue list to prioritize cross-probeable extra-layout-net rows before schematic-only missing-net rows for select-first actions.
- Changed SPICE schematic mismatch compares to open the SPICE Issues net-browser filter when browsable issue nets exist, while clean matches stay on All.
- Added a dedicated Select First SPICE Issue action so LVS-style compare issue rows can be cross-probed directly with the current browser search.
- Added a Clear SPICE Compare action in the net browser to drop the active LVS-style comparison report and reset SPICE-specific net filters.
- Added deterministic trace-state JSON export/import for the current trace selection, trace history, and route points, validating imported component IDs against current extracted connectivity.
- Added deterministic L2N database JSON export/import that bundles extracted connectivity, supported extracted devices, current trace selection/history, and route points, with recent-file reload integration.
- Added searchable Trace State summary rows to the net browser and Connectivity inspector so imported or current L2N state exposes selected trace net, trace-history count, pending route-point count, and highlight mode without opening the JSON.
- Added a one-shot Clear Trace State net-browser control for clearing the selected trace net, trace history, and pending route points together after tracing or L2N import.
- Extended trace-state and L2N JSON exchange to persist selected/history/off trace-highlight mode, with older trace-state JSON defaulting to Selected on import.
- Updated trace-state and L2N imports to switch the net browser to the Trace History filter when restored history is present, so imported traced nets are listed immediately.
- Added recent-file and reload-most-recent integration for trace-state JSON exchange, matching the L2N database reload workflow.
- Added imported connectivity source rows to the net browser and searchable Connectivity inspector so imported netlist/L2N data shows its source document, layout revision, and technology metadata.
- Added route-time known-net context so existing same-net or same-component geometry is treated as usable route context rather than as an obstacle.
- Added a searchable read-only 2D technology stack browser that surfaces DBU/grid, active layer GDS/text mapping, connectivity stack links, and DRC rule-family counts from the active technology with `key=value` selectors for technology, DRC, layer, mapping, and stack-link metadata.
- Added focused native-app coverage for selecting a component from the net browser, tracing a net from a canvas click, tracing between route points, trace-history search, netlist JSON round-trip, SPICE netlist export, SPICE schematic comparison, trace-state JSON round-trip, and combined L2N database JSON round-trip.
- Added first device-aware LVS slices: diffusion/poly crossings are extracted as MOS-like devices with labeled source/drain/gate/body terminals when side labels or overlapping well/body labels are present, isolated labeled-poly geometry is extracted as resistor-like devices, and labeled metal1/metal2 overlap without a via is extracted as a capacitor-like device; all are included in deterministic netlist JSON exchange, emitted as SPICE device lines, and compared against supported schematic device signatures.

Started closing KLayout-style geometry operation gaps:

- Added shapewise grow/shrink actions for selected current-view-cell rectangles, polygons, paths, and vias using a grid-derived sizing step.
- Added layer-wide grow/shrink actions for active-layer current-view-cell rectangles, polygons, paths, and vias using the same grid-derived sizing step.
- Added X-only and Y-only grow/shrink actions for selected and active-layer current-cell rectangles and polygons.
- Added selected and active-layer current-cell rectangle and polygon corner chamfering plus rounded-corner approximation that convert or keep shapes as editable polygons.
- Added selected current-cell shape and selected-instance left/right/top/bottom/center-X/center-Y alignment actions against the active layer's parent-cell shape bounds plus origin-axis centering actions, and active-layer current-cell shape alignment against a selected reference shape.
- Added an active-layer rectangle merge action that combines touching current-view-cell rectangles into exact non-overlapping union rectangles.
- Added a selected current-cell rectangle/polygon layer AND action that intersects active-layer current-cell rectangles and polygons with the selected region, decomposing non-convex selected regions into convex fragments.
- Added a selected current-cell rectangle/polygon layer OR action that writes active-layer current-cell union fragments for selected regions.
- Added selected current-cell rectangle/polygon layer NOT, selection NOT layer, and layer XOR actions that write active-layer rectangular or polygon fragments for selected regions.
- Added active-layer current-cell layer AND/OR/NOT/XOR against one-or-more clipboard rectangle/polygon operands, replacing active-layer geometry with undoable fragments while leaving clipboard source shapes intact.
- Added active-layer current-cell copy into the layout clipboard, plus selected current-cell shape versus one-or-more clipboard-shape AND/OR/NOT/XOR actions for rectangles and polygons, replacing the selected shape with undoable result fragments.
- Added selected-region and clipboard-region current-cell clip-cell actions that write active-layer clipped rectangle and polygon fragments into a new instanced child cell.
- Added selected current-cell shape copy, paste, duplicate, delete, rotate-90, mirror-X, and mirror-Y actions that edit geometry in the active viewed cell rather than leaking edits to the document top cell.
- Added current-cell selected-shape vertex/edge editing, including live drag history and vertex insertion/deletion replacement operations.
- Added current-cell selected-shape drag and alt-drag duplication, including undoable local shape move and duplicate history.
- Added current-cell-aware compact shape selection controls, including Next Shape cycling for local shapes in the active viewed cell.
- Added browser-search-driven replacement for matching shape names, shape property keys/values, layer names, label/ruler text, cell names, cell property keys/values, instance names, and instance property keys/values, with document/current-cell/visible-hierarchy scope controls and undo/redo support.
- Added shape/cell/instance/net browser select-first support so filtered searches, including property-selector searches such as `key=value`, can directly select or view the first matching browser result.
- Added focused native-app coverage for selected and active-layer grow/shrink including current-cell local shapes, axis-specific current-cell rectangle/polygon sizing, and non-convex polygons, selected and active-layer rectangle/polygon chamfering/rounding including current-cell local shapes and non-convex polygons, selected current-cell shape transforms, current-cell vertex/edge editing, current-cell shape drag/alt-drag, current-cell shape control selection, selected-shape, selected-instance, and active-layer-to-reference alignment, current-cell exact rectangle merging without L-shape overfill, selected-region current-cell layer OR/AND/NOT/selection-NOT-layer/XOR clipping including non-convex selected regions, current-cell active-layer clipboard booleans, current-cell selected-shape clipboard booleans including active-layer clipboard capture, multiple clipboard operands, and non-convex selected or clipboard polygons, selected-region and clipboard-region current-cell clip-cell creation, current-cell copy/paste/duplicate/delete, browser search/replace including shape and instance property keys/values, and undoable shape replacement.

Started closing KLayout-style hierarchy editing gaps:

- Added an undoable Flatten Instance action for selected document-top, current-cell, or deeper visible-occurrence instances, replacing the instance with transformed local shapes.
- Added an undoable Make Cell action for selected current-cell shapes, replacing the local shape with a local instance of a normalized child cell.
- Added undoable Resolve Array actions for selected document-top, current-cell, or deeper visible-occurrence instance arrays, plus current-cell, descendant-cell, and whole-document batch array resolving, replacing arrays with individual single instances.
- Added undoable Make Variant actions for selected document-top, current-cell, or deeper visible-occurrence instances, plus current-cell, descendant, and document-wide child-instance batches, shallow-copying target cells with fresh local shape/instance IDs and retargeting only those instance records.
- Added an undoable Duplicate Cell action for the current 2D view cell, shallow-copying local geometry and child instances into a standalone viewable cell with fresh local IDs.
- Added a guarded Delete Unused Cell action for the current 2D view cell, protecting the document top cell and cells that are still instanced.
- Added an undoable Delete Unused Cells batch action that removes cells unreachable from the document top, including unreachable subtrees, while preserving the live top hierarchy.
- Added an undoable Shallow Delete Cell action for the current 2D view cell, removing its parent references while preserving child cells.
- Added an undoable Deep Delete Cell action for the current 2D view cell, pruning descendant cells that are no longer referenced while preserving shared child cells.
- Added an undoable Complete Delete Cell action for the current 2D view cell, removing the full descendant subtree and any external references to deleted child cells.
- Added undoable Flatten Cell actions for the current 2D view cell, descendant-cell batches, and whole-document batches, replacing child instances with transformed local geometry while preserving references to edited cells.
- Added undoable Origin to Selection, exact browser-search DBU coordinate, and grid-derived Origin +/-X/Y actions for the current 2D view cell, shifting local geometry and compensating parent instances so placed geometry remains stable.
- Added undoable descendant leaf-cell origin-to-local-bounds batching, shifting local leaf geometry and compensating parent instances while preserving placed geometry.
- Added an undoable Move Shape Up action for selected current-cell local or deeper visible-occurrence source-cell shapes, materializing transformed copies in parent cell placements.
- Added an undoable Move Instance Up action for selected current-cell local or deeper visible-occurrence instances, materializing transformed parent-cell instances while preserving placed geometry.
- Added focused native-app coverage for flattening and undoing a top-level, current-cell, or deeper visible-occurrence instance, making and undoing a current-cell shape cell, making and undoing a document-top or deeper visible-occurrence selected-instance variant plus current-cell, descendant, and document-wide child-instance batch variants, flattening the current cell plus descendant-cell and whole-document batches, moving a current-cell or deeper visible-occurrence shape/instance up, setting top/current cell origins from selection, exact coordinates, grid-derived nudges, or descendant leaf-cell bounds, resolving and undoing top-level/current-cell/deeper visible-occurrence instance arrays plus current-cell, descendant-cell, and whole-document batch arrays, and deleting/undoing individual unused cells plus all-unused-cell batches.

Started closing KLayout-style view utility gaps:

- Added Bookmarks menu actions plus inspector summaries for saving, restoring, naming, per-slot clearing, and clear-all management for eight options-backed 2D layout view states, including pan, zoom, current view top cell, and hierarchy display depth range.
- Added Previous Layout View navigation that toggles back to the prior 2D view state after bookmark restores, pan/zoom changes, top-cell changes, hierarchy-depth changes, origin focus, or bounds focus.
- Added deterministic layout view bookmark JSON export/import for the eight named 2D bookmark slots.
- Added searchable view-bookmark inspector summaries with `key=value` selectors for current, previous, saved, and empty view slots by slot/name/state/top cell/zoom/pan/hierarchy metadata.
- Added Origin, Layout Bounds, and Selection focus actions backed by the current 2D view state rather than placeholder menu labels.
- Added File-menu deterministic PNG, RGBA, and portable PPM screenshot export using the existing snapshot renderer.
- Added File-menu deterministic layout JSON and workspace JSON save/load using the validated model serializers and native `target/glassworks-layout.json` / `target/glassworks-workspace.json` snapshot paths.
- Added File-menu deterministic app-session JSON save/load at `target/glassworks-session.json`, bundling workspace data, app options, active shell/view state, and the current 2D layout view.
- Added File-menu layout JSON import-as-cell, remapping an imported document into child cells and placing the imported root as an instance without replacing the current layout.
- Added File-menu layout JSON import-as-top-cell, remapping an imported document into extra standalone cells and switching the 2D view to the imported root without instantiating it in the document top cell.
- Added File-menu layout JSON merge, flattening an imported document into current top-level geometry with layer remapping and undo support.
- Added File-menu layout JSON hierarchy merge, splicing imported root-cell local shapes and child instances into the current top cell while preserving imported child-cell hierarchy.
- Added the same hierarchy-preserving root merge action for GDS, CIF, DXF, DEF, and LEF import documents through the shared validated import planner.
- Added File-menu GDS import-as-cell, GDS extra-top-cell import, and GDS merge actions that reuse the same layer mapping, import-offset, validation, and undoable import plans.
- Added File-menu CIF import-as-cell, CIF extra-top-cell import, and CIF merge actions through the same validated document-import path.
- Added File-menu DXF import-as-cell, DXF extra-top-cell import, and DXF merge actions through the same validated document-import path.
- Added gzip- and single-file-ZIP-aware loading for native workspace JSON, layout JSON load/import/merge, and GDS import paths.
- Added File-menu JSON import layer-mapping policies for preserving matching layer IDs, matching by layer name, or copying every source layer.
- Added File-menu JSON import layer-ID offset controls so newly imported or copied source layers can prefer shifted target layer IDs.
- Added File-menu import-offset controls that translate JSON/GDS/CIF/DXF/DEF import-as-cell placement and flattened merge geometry.
- Added File-menu recent entries for native app-session, workspace, layout JSON, and GDS exchange paths, persisted through app options with a configurable cap.
- Added File-menu reload for the most recent native app-session/workspace/layout/GDS exchange path, reusing the same validated loaders.
- Added persisted reference-image overlays with File-menu deterministic JSON exchange at `target/glassworks-reference-images.json`, recent-file tracking, and gzip-/single-file-ZIP-aware import.
- Added global, per-image, and bulk reference-image visibility controls, per-image focus, per-image opacity/order controls, per-image removal, searchable source/pixel-size/visibility/opacity/bounds/landmark inspector rows with `key=value` selectors, bulk reference-image clearing, translucent overlay rendering, landmark crosshairs, inspector rows, bulk/per-image landmark align/seed/fit/clear controls, and two-or-more-landmark axis-aligned alignment.
- Added a searchable measurement browser with live filter counts, active filter/search row-count titles, no-match selection status context, listed-ruler total-length/mode summaries, and endpoint, delta, length, angle, layer, source-cell, and occurrence property rows for ruler annotations.
- Added measurement-browser filters for selected rulers, active-layer rulers, and direct/horizontal/vertical/Manhattan ruler modes, plus ID/length/angle/mode/layer/source-cell sort controls.
- Added measurement-browser `key=value` selector search for reviewing ruler annotations by source cell, layer, mode, endpoints, deltas, lengths, angles, and bounds.
- Added persisted ruler drawing modes for direct, horizontal, vertical, and Manhattan measurement annotations, with mode controls and property rows.
- Added first-matching ruler selection, selected-ruler focus, selected-ruler deletion, and undoable visible-ruler cleanup from the measurement browser.
- Added focused native-app coverage for bookmark restore, previous-view restore, named bookmark option and exchange round-trips, Bookmarks menu enablement/clear/export/import actions, app-session and layout/workspace JSON round-trip, child-cell/top-cell/flattened-merge layout JSON import, recent file reopening/reload, reference-image exchange/alignment/render toggles, UI screenshot export, and measurement property browsing/cleanup.

Started closing KLayout-style marker review gaps:

- Added persisted DRC marker visited and important tags alongside hidden/waived review state.
- Selecting a DRC marker now marks it visited, and the marker browser exposes visited/important/noted/snapshot filters plus toggle and note actions.
- Added first-matching DRC marker selection from the marker browser so filtered/search marker lists can cross-probe without clicking a specific row.
- Added persisted DRC marker owner and signoff metadata with owner/signed-off filters, preset controls, property rows, and report JSON round-trip support.
- Added persisted DRC marker key/value tags with tagged filters, preset controls, property rows, search indexing, and report JSON round-trip support.
- Added custom DRC marker key/value tag apply/remove actions that use the browser search and replace fields for ad hoc marker metadata.
- Added active-report DRC marker tag clearing that removes key/value review tags while preserving other marker review state.
- Added undoable browser replacement for active-report DRC marker review notes, owners, signoffs, and tag keys/values without invalidating the active DRC report.
- Added `key=value` selector search for active-report DRC marker rule/state/geometry/review metadata and tag fields.
- Added user-tagged DRC marker category directory grouping, directory filtering, category/directory selector search, and a category tag preset for review-driven marker organization.
- Added KLayout `.lyrdb` imported category-part directory grouping so quoted category names containing `/`, `.`, or hyphens stay intact in marker directory rows and filters.
- Added search and listed-directory summaries to DRC marker directory inspector rows so selector-filtered category groups are visible outside the marker browser list.
- Added search and listed-marker summaries to DRC marker info rows so selector-filtered marker detail panes show their active match context.
- Added top-level DRC report-database summary and preview rows for listed reports, stored markers/issues, and current/stale report counts.
- Added active-report database context, aggregate stored-report summaries, and cross-probe target-count summaries to DRC marker info rows.
- Added selected-marker source-object kind, layer, and source-local bounds summaries to DRC marker info rows.
- Added DRC marker search and `key=value` selectors for source objects, source layers, source kinds, and source-local bounds.
- Added DRC marker sort controls for source objects, source layers, source kinds, and source-local bounds.
- Added source-object, source-layer, source-kind, source-local bounds, and cross-probe target summaries to selected DRC marker browser detail rows.
- Added top-level DRC marker summaries for listed source objects, source cells, source occurrences, and aggregate cross-probe targets.
- Added top-level DRC marker summaries for listed source layers, source kinds, and source-local bounds.
- Added top-level DRC marker label summaries for listed source objects, source cells, and source occurrences.
- Added top-level DRC marker summaries for listed rules, categories, and directories.
- Added top-level DRC marker summaries for listed marker IDs and message previews.
- Added top-level DRC marker geometry rollups for listed marker bounds, area, required values, and actual values.
- Added top-level DRC marker review rollups for listed review states, signoff audit, tags, and snapshots.
- Added accumulated per-party DRC marker signoff records with selected-marker summaries, selector search, browser replacement, report JSON round trips, and Calibre/RVE signoff seeding.
- Added role and deterministic review-stamp metadata to DRC marker signoff records, including `signoff_role`/`signoff_at` selector search and Calibre/RVE metadata import.
- Added native-browser wall-clock seconds to newly recorded DRC marker signoff audit stamps while preserving the layout-revision prefix for deterministic review sorting/search.
- Added DRC marker sort controls for signoff status, signoff role, and newest signoff stamp so review queues can be scanned by signoff metadata.
- Added DRC marker review-status filters for needs-review, accepted, and rejected signoffs, matching both current signoff metadata and accumulated per-party records.
- Added listed DRC marker review-state summaries to the marker info pane so filtered review queues expose hidden/waived/visited/signoff/tag/snapshot counts.
- Added listed DRC marker signoff-audit summaries to the marker info pane so filtered review queues expose signoff record, reviewer-party, status, role, and newest-stamp rollups.
- Added listed DRC marker tag summaries to the marker info pane so filtered review queues expose tagged-marker counts, tag-key counts, and compact key/value tag previews.
- Added listed DRC marker snapshot summaries to the marker info pane so filtered review queues expose snapshot counts plus format and size rollups.
- Added shared-search filtering, `key=value` selectors, first-match selection, per-report delete controls, listed/total count titles, and no-match context to the active DRC report list.
- Added marker-aware active DRC report-list search so stored reports can be found by contained marker rules, source objects, directories, tags, and signoff metadata.
- Added selected DRC marker RGBA/PNG snapshot export that focuses the marker, writes a deterministic native snapshot, stores screenshot path/bounds/size/format metadata as marker tags for report JSON exchange, and exposes that snapshot metadata in marker detail rows.
- Added thumbnail-capable, count/search-aware active-report DRC marker snapshot gallery rows with searched-empty context and selectable entries that cross-probe back to the source marker.
- Added selected and active-report DRC marker snapshot metadata clearing that removes screenshot tags without deleting unrelated review tags.
- Added active-report bulk clearing for persisted DRC marker review state, scoped to the markers in the current report without deleting report history.
- Added a selected-region DRC action that runs the current rule deck and caches only markers intersecting a selected current-cell rectangle or polygon, decomposing non-convex selected regions into convex fragments.
- Added a current-cell DRC action that runs the same physical rule deck with the active layout cell as the DRC root.
- Added deterministic DRC report JSON export/import for the current run, including marker review state, without rerunning DRC on export.
- Added Calibre/RVE-style text marker import for external polygon, rectangle, edge, point, and circle marker geometry into the current DRC marker browser.
- Added an undoable DRC marker output action that writes active markers into ordinary annotation-layer geometry.
- Added DRC marker rule-family category grouping and filters for grid, width, area, spacing, enclosure, overlap, and other markers.
- Added DRC marker rule-path directory rows/filter controls and a selected-marker info pane with message, stable key, occurrence, geometry, and review metadata.
- Added selected-marker geometry/object info rows for center, size, area, shape IDs, and occurrence labels across the DRC marker info and property panes.
- Added selected-marker source-object cross-probe controls that select marker source shapes, source cells, or exact source occurrences from the DRC marker browser.
- Added selected-marker source-occurrence cross-probe buttons that restore the exact hierarchical occurrence path, active layer, and top-cell view for markers emitted from instances.
- Added selected-marker source-cell cross-probe buttons that switch to the owning source cell and select the local marker shape when possible.
- Added DRC marker category, directory, source-cell, size, area, required, and actual sort controls alongside ID/rule/state sorting.
- Added DRC marker source-cell rows and `source_cell=` selector search for reviewing hierarchical markers by owning source cell.
- Added DRC marker signoff signer/note metadata capture, rows, selector search, search-replace coverage, and JSON round-trip support.
- Added a selected-shape DRC marker filter for reverse cross-probing from a selected layout object back to matching markers.
- Added an active-layer DRC marker filter for reviewing only markers that touch the currently active layout layer.
- Added live DRC marker filter counts plus active filter/category/directory/search row-count titles and select-first no-match context.
- Added a bounded in-memory DRC report list for recent full/cell/region runs and imported marker reports, with active-report selection, delete, and clear controls.
- Added deterministic DRC report-database JSON export/import for the whole bounded report list, preserving active-report selection and marker review state.
- Added DRC report-database source metadata on imported reports with report-browser `source=`/`database=` selector search and stored-report source summaries.
- Added append-load support for DRC report databases so multiple source-backed report databases can coexist in the bounded report browser.
- Added DRC report database source-group controls and inspector summaries so appended/imported report sources can be selected directly.
- Added selected-marker matching-report/source summaries for loaded DRC report databases.
- Added source-level DRC report database removal so one loaded report source can be closed without clearing all reports.
- Added gzip- and single-file-ZIP-aware import for DRC decks, DRC reports, DRC report databases including KLayout `.lyrdb` XML payloads, and Calibre/RVE text marker files.
- Added first-class Tools-menu and DRC marker-panel KLayout `.lyrdb` report database export/import/append actions alongside the JSON report-database exchange actions.
- Added KLayout `.lyrdb` XML report database import/append for common marker values (`box`, `edge`, `edge-pair`, `point`, `polygon`, `path` with begin/end extensions, `label`, legacy `rect`/`rectangle` and `edge_pair` aliases, legacy geometric `text`/DText, KLayout `text`/`float` scalar values including quoted leading/trailing, empty, and whitespace-only text payloads, and legacy `string` scalar imports) plus text-only diagnostic items, KLayout-compatible text-encoded item references and raw values, item category/cell/tags/tag-description/comment/visited/multiplicity metadata including KLayout-quoted item tag lists, original tag-name preservation for typed re-export, quoted and backslash-escaped category-path part preservation, declared category-description and declared tag-description metadata preservation, root description/top-cell/original-file/generator metadata preservation including native empty root elements, cell layout-name/reference metadata and declared cell metadata preservation including literal colon-containing cell names that collide with displayed cell variants, multiple/scaled/arbitrary-angle cell-reference transform paths represented as separate imported marker rows in the report top-cell context, current-layout hierarchy fallback when the RDB cell graph is incomplete, unique unqualified declared cell-variant fallback, supported tagged geometry value preservation for typed re-export, tagged item-reference value preservation, ordered item-value sequence preservation, KLayout-native `[#tag]`/`[#'tag name']` value-tag prefix preservation, and tagged raw preservation of unsupported item values attached to imported markers.
- Added KLayout `.lyrdb` native `#'tag name'` item tag-list import so KLayout-authored item tags with spaces preserve their clean original tag names and descriptions on re-export.
- Updated KLayout `.lyrdb` export to emit item tag lists with KLayout's native `#tag`/`#'tag name'` syntax.
- Updated KLayout `.lyrdb` value-tag import/export to normalize native unquoted word tags such as `[#review_lane]` and emit KLayout's `[#tag]`/`[#'tag name']` form for value-level tags.
- Added KLayout `.lyrdb` item-tag import for `#hidden`/`#hide` so hidden marker review state round-trips through the KLayout tag surface instead of coming back as a custom tag.
- Updated KLayout `.lyrdb` cell-reference metadata import/export to accept KLayout-native `<ref>` nodes, retain legacy `<reference>` imports, and emit native `<ref>` nodes on export.
- Added KLayout `.lyrdb` XML report database export when saving report databases to `.lyrdb` or gzip-compressed `.lyrdb.gz` paths, preserving marker bounds, messages, category/cell metadata including the empty-name All cells declaration for cell-less diagnostic items and literal colon-containing cell names separately from KLayout variants even when both share the same displayed `name:variant` text, original imported tag identifiers such as hyphenated tag names and escaped apostrophes through KLayout-quoted item tag lists, quoted category names containing `/`, `.`, hyphens, or apostrophes, root description/top-cell/original-file/generator metadata including native empty root elements, declared category-description metadata while leaving generated no-description categories as native empty `<description/>` elements with KLayout-style leaf `<categories>` containers, declared tag-description metadata, cell layout-name metadata including native empty `<layout-name/>` elements, KLayout-native cell-reference metadata with expanded empty `<references>` containers and native `<ref>` entries, declared cell metadata, original supported `box`/`edge`/`edge-pair`/`point`/`polygon`/`path`/`label` value payloads from imported RDB markers, ordered item-value sequences from imported RDB markers with legacy `string` scalars normalized to KLayout `text`, native unquoted simple text scalar payloads plus quoted leading/trailing, empty, and whitespace-only text payloads, legacy geometric `text` values normalized to KLayout `label`, legacy `rect`/`rectangle` aliases normalized to KLayout `box`, legacy `edge_pair` normalized to `edge-pair`, point values normalized to zero-size KLayout-compatible `box` values, item references normalized to KLayout-compatible `text: 'reference: ...'` payloads, and unsupported raw values normalized to KLayout-compatible `text: 'glassworks-raw-value: ...'` payloads while retaining KLayout-native escaped `[#tag]`/`[#'tag name']` value-tag prefixes, tagged geometry values, tagged item-reference metadata, tagged scalar item values, tagged raw item values, visited/hidden/important/waived/custom tag state with native empty item `<tags/>` output, item multiplicity, comments, diagnostic text items, and preserved raw item values.
- Added KLayout `.lyrdb` embedded marker-image preservation, storing imported item image payloads and exporting embedded image payloads from imported RDB images or existing marker snapshot files.
- Surfaced imported KLayout `.lyrdb` embedded marker images in marker snapshot labels, filters, summaries, and snapshot gallery entries.
- Added canvas-backed DRC marker snapshot thumbnail rendering for native-window and snapshot-render paths, decoding embedded `.lyrdb` PNG image payloads plus PNG/RGBA marker snapshot files.
- Added Calibre/RVE order-independent keyed geometry coordinate import for marker lines such as `x1=... y1=... x2=... y2=...`, origin-plus-width/height rectangles, and radius-first circles.
- Added Calibre/RVE keyed center-plus-size rectangle parsing for forms such as `center=(x,y) width=... height=...`.
- Added Calibre/RVE paired keyed coordinate import for compact tuples such as `ll=(x,y) ur=(x,y)` rectangles, `center=(x,y) r=...` circles, and `vertex0=(x,y)` polygons.
- Added Calibre/RVE alternate keyed geometry tuple import for `ul/lr` rectangle corners, `start/end` edge endpoints, `loc/position` point centers, `rad` circle radius aliases, and `xc/yc`/`centre_x` center-size rectangle aliases.
- Added Calibre/RVE tuple-valued rectangle size/corner import for forms such as `origin=(x,y) size=(w,h)`, `center=(x,y) size=(w,h)`, and `min/max` or `lower/upper` corner tuples.
- Added Calibre/RVE bracketed and underscore-indexed keyed coordinate import for forms such as `x[0]=... y[0]=...`, `x_1=... y_1=...`, `vertex[1]=(x,y)`, and bracketed coordinate tuples like `center=[x,y]`.
- Added Calibre/RVE keyed geometry-kind row import for forms such as `kind=rect ...`, `geometry_type: polygon ...`, `shape = point ...`, `1: type=edge ...`, and pending `marker_type: rectangle` coordinate rows.
- Added Calibre/RVE labeled multi-line coordinate row import for pending geometry rows such as `coordinate: x y`, `vertex 1: x y`, `point[3]: x y`, `start: x y`, `end: x y`, and `location: x y`.
- Added Calibre/RVE pending keyed coordinate row import for multi-line geometry rows such as `x 10 y 20`, `x: 10 y: 20`, `x=10 y=20`, and `cx 3000 cy 4000 radius 25nm`.
- Added Calibre/RVE long-form metric unit aliases and scientific-notation parsing for coordinates and measurements, including `micrometer`, `micrometre`, `nanometre`, and `millimeter`/`mm` forms.
- Added Calibre/RVE ASCII result-database signature parsing for top-cell precision headers, rule count/timestamp rows, `p`/`e` ordinal/count records, `CN` cell-space transforms, and attached result-property lines.
- Added Calibre/RVE keyed circle diameter parsing for forms such as `diameter=...`, `diam=...`, `dia=...`, and `d=...`, normalizing them to marker radii.
- Added Calibre/RVE marker geometry parsing for separated numeric/unit coordinate tokens such as `1 um` in both positional and keyed geometry fields.
- Added Calibre/RVE marker geometry parsing for separated keyed fields such as `x 3 um`, `y = 4 um`, and `height: 50 nm`.
- Added Calibre/RVE metadata and measurement parsing for separated separators such as `Cell = TOP`, `Layer : M1`, `@ owner = ...`, and `Required = ... Actual : ...`.
- Added Calibre/RVE multi-word metadata label parsing for context fields such as `Cell Name: TOP`, `Source Cell = BLOCK`, `Layer Name: M1`, and `Source Layer = M1_PIN`.
- Added Calibre/RVE `Rule Check Name:` rule-label aliases and result/marker/violation record delimiter handling so numbered RVE result headers do not replace the active rule name.
- Added Calibre/RVE counted polygon import for numbered vertex rows such as `1: x y`, so row ordinals do not distort marker bounds or become stray rule names.
- Added Calibre/RVE geometry keyword aliases and trailing-colon geometry labels such as `bbox: ...` and `poly ...`.
- Added Calibre/RVE attached geometry-label parsing for no-space forms such as `bbox=...` and `points:...`.
- Added Calibre/RVE plural and short geometry aliases such as `points: ...`, `circ: ...`, `rectangles`, and `segments`.
- Added Calibre/RVE pending rectangle/edge coordinate collection for numbered rows such as `1: x y`, matching the counted polygon row-index handling.
- Added Calibre/RVE pending point/circle coordinate collection for numbered rows such as `1: x y` and `1: cx cy r`, so row ordinals do not skew point centers or circle bounds.
- Added Calibre/RVE whitespace-separated numbered coordinate row import for rows such as `1 x y`, matching the punctuated `1: x y` row-index handling.
- Added Calibre/RVE numbered inline geometry row import for result-table rows such as `1: rect ...` and `2. edge ...`, so row ordinals do not become bogus rule names.
- Added Calibre/RVE `@ tag`/`@ tags` item parsing for colon-separated key/value forms such as `defect_class: "edge, jog"` while preserving quoted delimiters.
- Added Calibre/RVE `@ tag`/`@ tags` item parsing for whitespace-separated key/value forms such as `review_lane layout` and semicolon-separated `inspection_phase final; disposition accepted`.
- Added Calibre/RVE quoted rule-label normalization so labeled rule names such as `RuleCheck: "QUOTED RULE"` do not retain quote wrappers in rule IDs or default marker messages.
- Added app-session save/load support for the bounded DRC report list and active report selection, so marker review sessions survive ordinary session restore without a separate report-database exchange.
- Added recent-file and reload-most-recent integration for DRC report JSON, DRC report-database JSON, and Calibre/RVE marker imports.
- Added lightweight Calibre/RVE `@ owner/note/signoff/tag` review metadata import so external marker text can seed owner, note, signoff, category, and custom marker tags.
- Added Calibre/RVE quoted metadata value handling so external review fields do not retain surrounding quotes.
- Added Calibre/RVE approval metadata aliases such as `approval_status`, `approved_by`, `approval_note`, `approval_role`, and `approval_at` so external review exports seed the same per-party signoff records.
- Added Calibre/RVE `@ tags` multi-value metadata handling for semicolon- and comma-separated external review tags while preserving quoted delimiter characters inside tag values.
- Added Calibre/RVE direct review tag metadata for fields such as `severity`, `classification`, `source`, `run_id`, `tool_version`, and `deck_name` so common external review attributes stay searchable instead of replacing marker messages.
- Added Calibre/RVE DBU, um, and nm coordinate suffix parsing so mixed-unit external marker exports import to deterministic DBU marker bounds.
- Added Calibre/RVE counted multi-line marker geometry collection so exports with `p 4` followed by coordinate rows import as one marker instead of skipped coordinate lines.
- Added Calibre/RVE uncounted multi-line polygon collection so `polygon` followed by coordinate rows finalizes when the next marker record begins.
- Added skipped-line accounting and capped warnings for incomplete Calibre/RVE multi-line marker geometry so partial external records are visible in import diagnostics.
- Surfaced the first Calibre/RVE parser warning in native import status messages so partial external marker records are visible from the UI import flow.
- Persisted Calibre/RVE parser warnings as imported report warning findings so diagnostics remain visible with the marker report after status text changes.
- Kept marker browsing, category rows, directory rows, and marker summaries active when an imported report has non-fatal warning diagnostics, and exposed report warning/error summaries in marker info rows and report search selectors.
- Preserved warning-only Calibre/RVE imports as diagnostic-only report history entries so malformed external marker files leave durable parser feedback instead of only transient status text.
- Added DRC report-browser free-text and `key=value` selector search over stored diagnostic messages and severities, including Calibre/RVE parser warning reports.
- Added aggregate ancestor rows and descendant filtering for DRC marker directory paths so rule and user category tags behave more like hierarchical marker directories.
- Added selected-marker direct tag removal controls so individual custom marker tags can be edited without retyping the key into browser search.
- Added Calibre/RVE labeled `RuleCheck:`, `Message:`, `Rule Description:`, `Cell:`, `Layer:`, and `Datatype:` handling so common non-`@` RVE record fields seed rule names, marker messages, and searchable context tags.
- Added Calibre/RVE labeled and combined `Required:`/`Actual:` handling, including `key=value` forms and DBU/um/nm suffixes, so imported external markers populate the existing required/actual marker rows and sort/search surfaces.
- Added Calibre/RVE free-form `@` message accumulation so multi-line external marker descriptions are preserved instead of being overwritten by the last comment line.
- Added focused layout-model and native-app coverage for marker-state serialization, operation replay, selected-region and current-cell DRC including current-cell selected regions, visited/important/noted/owned/signed-off/tagged marker browser workflows, selected and active-report bulk marker-state clearing, active-report marker-tag clearing, selected-marker geometry/object info rows, custom marker tags and category-tag grouping, marker snapshot export plus selected and active-report metadata-clearing behavior, rule-category marker filtering, DRC report JSON round-trip, DRC report-database JSON round-trip, Calibre/RVE marker import, marker annotation output, and report-list selection/delete/clear behavior.

Started closing KLayout-style PCell/library gaps:

- Added a 2D cell-browser Library surface for a via-array primitive with editable row, column, via-size, and pitch parameters.
- The generator uses the active technology's via1/metal1/metal2 layers and places the generated array as a current-cell reusable instance.
- Added stored cell properties for generated library macro cells, including the source macro and parameter values, and preserved those properties through layout JSON and Loro object materialization.
- Added load/update actions so a selected generated via-array macro cell can restore its saved parameters into the controls and regenerate its local via geometry in place while preserving existing instances.
- Added selected guide-shape conversion so a current-cell rectangle, polygon, path, via, label, or measurement can be replaced by a placed via-array macro sized from its bounds.
- Added a detach-to-static action that clears generated macro metadata from a selected library cell while keeping its geometry and existing instances.
- Added four options-backed via-array library preset slots with save/load/place/clear controls for a minimal persistent reusable-macro catalog.
- Added deterministic JSON export/import for the via-array library preset catalog.
- Added a Cell Browser Library filter that shows generated macro cells as a minimal reusable-library view.
- Kept the generated primitive as a normal static cell with undo/redo support, leaving dynamic editable PCells, richer guide-to-parameter policies, general PCell-to-static workflows, and full external libraries beyond preset catalog exchange as remaining KLayout-grade gaps.
- Added focused native-app coverage proving the library controls exist, parameter edits drive generated geometry, the generated cell is reusable, the placed occurrence is selected, and undo removes generated objects.

## Immediate Next Checkpoint

The roadmap milestones and first browser/net-tracing/geometry-operation/hierarchy-editing/view-utility/marker-review/library-macro slices are implemented. The next task should continue with the remaining KLayout 2D gaps rather than treating the gap list as complete.

Concrete options:

- Completion audit: verify each milestone against code, docs, tests, screenshots, and remaining known gaps.
- Collaboration smoke test: run two native clients and two `trunk serve` browser clients against one sync server, edit from each, disconnect/reconnect one client, and confirm convergence.
- Connectivity polish: deepen trace/path visualization and richer net-component routing policies.
- Format polish: add a tiny checked-in KLayout-created GDS fixture if available.
- Verification polish: expand DRC/report browsing beyond the current explicit Run DRC workflow.
- Hierarchy polish: add richer cell-management actions.
- PCell polish: add dynamic PCells, full external library catalogs, or arbitrary guiding-shape conversion.

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
- No known panic in `cargo run -p native_app`.

## Non-Goals For Now

These are real EDA concerns, but they should not distract the near-term build:

- Full foundry-accurate process design kits.
- Full OASIS support before GDSII basics.
- Full CRDT editing semantics before operation-order sync is proven.
- Transistor-level LVS before connectivity extraction exists.
- Hand-tuned billion-triangle rendering before hierarchy and tile streaming are in place.

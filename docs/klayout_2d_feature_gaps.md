# KLayout 2D Layout Feature Gaps

Date: 2026-05-18

This compares Glassworks's current 2D layout editor against KLayout's documented layout-editor feature set. It is scoped to 2D layout authoring, viewing, verification, import/export, and layout automation. It does not treat Glassworks's fab workflow, MES, 3D stack, or collaboration modules as KLayout gaps.

Glassworks already has a useful 2D baseline: layer visibility, rendered per-layer fill/line display styles, options-backed fixed hierarchical layer group and used/empty row filtering, options-backed per-layer hierarchy depth overrides, drawing rectangles/polygons/paths/vias/labels/direct-horizontal-vertical-Manhattan measurements with listed-ruler aggregate summaries, selected-ruler focus/deletion, and undoable visible-ruler cleanup, vertex and edge editing, selected-object properties, selected-shape, selected-instance, and current-cell custom key/value property set/remove controls, stored shape/cell/instance key/value metadata, hierarchy/cells/instances/arrays, a small parameterized via-array library macro, GDSII subset import/export including label/TEXT records, orthogonal SREF/AREF rotation/reflection round-tripping, PATH square/custom end-extension import, shape-name/net-ID/stored-shape-and-instance-key-value property round-tripping, and BOX import as geometry, File-menu exchange, and gzip-aware import, flat CIF subset import/export including File-menu exchange and gzip-aware import, flat DXF subset import/export including File-menu exchange and gzip-aware import, flat DEF subset import/export including File-menu exchange and gzip-aware import, flat LEF macro subset import/export including File-menu exchange and gzip-aware import, File-menu app-session JSON plus layout JSON and workspace JSON save/load through validated model serializers with gzip-aware loading, File-menu JSON, GDS, CIF, DXF, and DEF import as a child-cell instance, extra top cell, or flattened merge, selectable JSON/GDS/CIF/DXF/DEF import layer mapping by ID/name/copy policy with a layer-ID offset for newly imported/copy layers, import-offset controls for JSON/GDS/CIF/DXF/DEF import-as-cell and flattened merge placement, recent native session/layout/workspace/GDS/CIF/DXF/DEF/LEF/reference-image/DRC-deck/DRC-report/DRC-database/Calibre-RVE/L2N-database exchange entries, browser-driven replacement for shape names, shape property keys/values, layer names, label/ruler text, cell names, cell property keys/values, instance names, and instance property keys/values with document/current-cell/visible-hierarchy scope controls, deterministic app-session, named layer-set, layout-view-bookmark, and reference-image JSON exchange with per-image visibility, focus, opacity, order, and removal controls, basic data-driven DRC with importable/exportable custom JSON decks, axis-aligned min-edge-spacing checks, bounds-based named `AND`/`OR`/`NOT` derived layers with derived min/max-width, min/max-area, min-spacing, min-edge-spacing, and forbidden-overlap checks, plus full-layout/current-cell/selected-region runs, an active-report DRC marker browser with bounded session-restored report history, rule-family grouping, rule-path directory rows, selected-marker info rows, persisted review state/notes/owner/signoff/preset-and-custom key-value-tag metadata, selected-marker RGBA snapshot export with path/bounds/size marker tags, active-report and report-database JSON exchange, Calibre/RVE-style text marker import, and active-marker annotation-layer geometry output, connectivity extraction with deterministic extracted-netlist JSON, simple MOS-like diffusion/poly with overlapping well/body-label bulk inference, isolated labeled-poly resistor-like, and labeled metal-overlap capacitor-like device extraction, SPICE-style `.subckt` export, supported-device SPICE schematic comparison, trace-state JSON exchange, and combined L2N database JSON exchange, explicit route/DRC actions, individual/immediate-child/descendant view-only cell hide/show controls, and a small named/options-backed layout view bookmark set. The gaps below are things KLayout has that Glassworks does not yet have or only covers in a narrow demo form.

The 2D side panel and inspector also include a technology stack browser for the active built-in technology, covering DBU/grid, active-layer GDS/text mapping, connectivity links, active-layer stack participation, and DRC rule-family counts. The active built-in technology can be switched from the Tools menu and drives document layer definitions, grid, connectivity extraction, GDS mapping, netlist metadata, and the built-in DRC deck while preserving imported/custom extra layers. Individual connectivity stack links can also be toggled for trace/net extraction experiments without changing layer visibility or DRC rules.

## High-Value Missing Features

### Rich Layout File Support

- OASIS read/write.
- Fuller LEF beyond Glassworks's current flat macro subset, fuller DEF beyond Glassworks's current flat DEF subset, fuller GDSII compatibility beyond Glassworks's current hierarchy/AREF/path-extension/path/label/oriented-reference/property-name-net-shape-and-instance-key-value/BOX-import subset, fuller CIF beyond Glassworks's current flat CIF subset, fuller DXF beyond Glassworks's current flat ASCII line/polyline/text/circle/arc subset, compressed layout loading beyond Glassworks's current gzip-aware app-session/layout JSON/workspace JSON/GDS/CIF/DXF/DEF/LEF/reference-image load paths, and richer reader options.
- Multi-file loading into separate views or the same panel.
- Drag/drop file loading, URL loading, and richer file dialogs beyond Glassworks's current deterministic File-menu app-session JSON, layout JSON, import-layout-as-cell JSON/GDS/CIF/DXF/DEF, import-extra-top-cell JSON/GDS/CIF/DXF/DEF, merge-layout JSON/GDS/CIF/DXF/DEF, GDS/CIF/DXF/DEF/LEF/reference-image exchange, gzip-aware app-session/workspace/layout/GDS/CIF/DXF/DEF/LEF/reference-image loading, workspace JSON snapshot paths, reload-most-recent action, and options-backed recent entries for those native exchange paths.
- Importing another layout into the current layout beyond Glassworks's current JSON/GDS/CIF/DXF/DEF-as-remapped-child-cell instance, extra-top-cell, and flattened top-level merge workflows, including selectable instantiate, richer top-cell handling, and merge-hierarchy modes.
- Richer reader-specific import mapping beyond Glassworks's current selectable JSON/GDS/CIF/DXF/DEF import layer mapping by ID/name/copy policy, JSON/GDS/CIF/DXF/DEF layer-ID offset for newly imported/copy layers, and File-menu import-offset placement transform controls.

KLayout source: [Loading A File](https://www.klayout.org/downloads/master/doc-qt4/manual/loading.html), [Import Other Layout Files](https://www.klayout.org/downloads/master/doc-qt5/manual/import_layout.html)

### Hierarchy Viewing And Navigation

- Navigate and manage cells from a richer tree-structured hierarchy browser. Glassworks now has root-cell-aware 2D rendering/picking, numeric min/max hierarchy display depths, parent/child context actions, selected-instance descend, a flat all-cell browser with current/used/unused/library/property-bearing/empty/leaf/branch/parent/child quick filters and name/id/shape-count/instance-count sort controls, and a collapsible hierarchy tree with bulk expand/collapse controls for choosing the 2D view top cell, but not KLayout's full hierarchy browser feature set.
- Hide/show cell workflows beyond Glassworks's current view-only individual cell, immediate-child group, descendant group, and show-all hidden-cell controls in the 2D hierarchy tree.
- Configure cell lists and browse cells with richer hierarchy controls beyond Glassworks's current flat browser filters, leaf/branch/current-parent/current-child filters, sort controls, and collapsible hierarchy tree with bulk expand/collapse controls.

KLayout source: [Choosing A Cell](https://klayout.org/downloads/master/doc-qt5/manual/cell.html), [Choosing A Hierarchy Depth](https://www.klayout.org/downloads/master/doc-qt5/manual/hier.html), [KLayout Basics](https://www.klayout.org/downloads/master/doc-qt5/manual/basic.html)

### Advanced Hierarchy Editing

- Flatten selected instances beyond Glassworks's current undoable document-top/current-cell one-level or deep instance flattening, including deeper nested occurrence selections and richer depth policies.
- Flatten whole cells beyond Glassworks's current undoable one-level or deep current-view-cell flattening, including batch flatten workflows.
- Move selected shapes or instances up in hierarchy beyond Glassworks's current undoable local-shape and local-instance move-up actions, including nested selections and richer multi-parent/cell-variant choices.
- Resolve arrays into editable individual instances beyond Glassworks's current selected document-top/current-cell array resolve action.
- Create cell variants beyond Glassworks's current shallow selected-instance variant action and standalone current-cell duplication, including deeper variant policies and richer occurrence editing workflows.
- Delete cells beyond Glassworks's current guarded unused-cell, shallow current-cell, unshared-subtree deep current-cell, and full-subtree complete current-cell delete actions, including richer batch policies and multi-cell management.
- Adjust cell origins beyond Glassworks's current undoable origin-to-selection, browser-search exact DBU coordinate, and grid-derived +/-X/Y origin actions for the current 2D view cell, including richer batch workflows.

Glassworks has basic create-cell, place-instance, transform, array metadata, instance deletion, shallow selected-instance cell variants, standalone current-cell duplication, document-top/current-cell one-level flattening that promotes nested instances, document-top/current-cell deep instance flattening into local shapes, current-view-cell one-level or deep flattening, origin-to-selection, exact browser-search DBU coordinate origins, and grid-derived origin nudges for the current view cell, local-shape and local-instance move-up into parent placements, document-top/current-cell array resolving, guarded unused-cell deletion, shallow current-cell deletion that removes references while preserving child cells, deep current-cell deletion that prunes unshared descendants while preserving descendants referenced elsewhere, and complete current-cell deletion that removes the full descendant subtree and external references, but not these richer KLayout hierarchy workflows.

KLayout source: [Advanced Editing Operations](https://www.klayout.org/downloads/master/doc-qt5/manual/editor_advanced.html), [Hierarchical Operations](https://klayout.org/downloads/master/doc-qt5/manual/hier_ops.html), [Resolving Arrays](https://www.klayout.org/downloads/master/doc-qt5/manual/resolve_arrays.html), [Delete A Cell](https://www.klayout.org/downloads/master/doc-qt5/manual/del_cell.html)

### Geometry Operations

- Layer boolean operations beyond Glassworks's current active-layer top-level exact rectangle union merge and selected rectangle/polygon active-layer OR/AND/NOT/selection-NOT-layer/XOR fragment workflows: arbitrary layer operands, hierarchy-aware output, and CAD-grade polygon boolean merging beyond selected-region decomposition.
- Shapewise boolean operations beyond Glassworks's current selected-shape versus clipboard-shape rectangle/polygon AND/OR/NOT/XOR workflow, including true multi-selected-shape operands, hierarchy-aware output, and CAD-grade polygon boolean merging beyond selected-shape decomposition.
- Layer sizing and full shapewise sizing beyond Glassworks's current selected/active-layer top-level rectangle/polygon/path/via grow-shrink actions, including hierarchy-aware sizing, split/merged offset outputs, and richer sizing modes.
- Object alignment tools beyond Glassworks's current selected-shape edge/center alignment against active-layer bounds and origin-axis centering.
- Corner rounding and smoothing beyond Glassworks's current selected rectangle/polygon corner chamfer and rounded-corner approximation operations, including smoother curve generation, hierarchy-aware output, and batch workflows.
- Clip creation from regions beyond Glassworks's current selected rectangle/polygon active-layer rectangle-or-polygon clip-cell action.
- Search and replace beyond Glassworks's current browser-driven replacement for shape names, shape property keys/values, layer names, label/ruler text, cell names, cell property keys/values, instance names, and instance property keys/values with document/current-cell/visible-hierarchy scope controls, including richer object properties and cross-layout workflows.

Glassworks currently has direct manual editing, transforms, selected-shape and active-layer top-level rectangle/polygon/path/via grow/shrink operations, selected rectangle/polygon corner chamfering and rounded-corner approximation, selected-shape edge/center alignment against active-layer bounds plus origin-axis centering, active-layer top-level exact rectangle union merging, selected rectangle/polygon active-layer OR/AND/NOT/XOR plus selection-NOT-layer clipping through convex selected-region decomposition, selected-shape versus clipboard-shape rectangle/polygon AND/OR/NOT/XOR through convex decomposition, selected rectangle/polygon active-layer clip-cell creation, and undoable browser-search replacement for matching shape names, shape property keys/values, layer names, label/ruler text, cell names, cell property keys/values, instance names, and instance property keys/values with document/current-cell/visible-hierarchy scope controls, but not CAD-grade batch geometry derivation tools.

KLayout source: [Advanced Editing Operations](https://www.klayout.org/downloads/master/doc-qt5/manual/editor_advanced.html), [Geometry API](https://www.klayout.org/downloads/master/doc-qt5/programming/geometry_api.html)

### PCells And Libraries

- Parameterized cells with editable parameters.
- General PCell conversion from arbitrary guiding shapes beyond Glassworks's current bounds-based via-array guide conversion.
- General conversion from PCells to static normal cells.
- A library view and reusable layout libraries.

Glassworks has ordinary cells and instances plus a small via-array library macro with editable row, column, size, and pitch parameters that creates and places a reusable static child cell. Generated macro cells now carry stored cell properties describing the source macro and parameter values, selected macro cells can load their parameters back into the generator and regenerate their local geometry in place, and the cell browser has a Library filter for those generated cells. A selected top-level rectangle, polygon, path, via, label, or measurement can also be converted into a placed via-array macro sized from the guide bounds, a generated macro cell can be detached into an ordinary static cell by clearing its macro metadata while preserving geometry and instances, and four options-backed via-array preset slots with deterministic JSON exchange form a tiny persistent reusable-macro catalog. This is still a narrow static-cell macro flow rather than general model-level PCells. Glassworks still does not have broader dynamic PCell regeneration, richer guide-to-parameter policies, general PCell-to-static conversion workflows, or a full reusable library system with external catalogs and multiple primitive types.

KLayout source: [PCell Operations](https://www.klayout.org/downloads/master/doc-qt5/manual/pcell_operations.html), [KLayout Basics](https://www.klayout.org/downloads/master/doc-qt5/manual/basic.html)

### Full DRC Engine

- Scriptable DRC deck language integrated with the macro/script environment beyond Glassworks's current editable JSON DRC deck exchange.
- Hierarchical and region-constrained DRC operation beyond Glassworks's current full-layout run, current-view-cell root run, and selected top-level rectangle/polygon region filter.
- Cell filtering and local checks beyond Glassworks's current active layout-cell root selection.
- Rich output to report databases or generated layout layers beyond Glassworks's current DRC report JSON and active-marker annotation-layer geometry output.
- Broader geometry predicates, edge-pair checks beyond Glassworks's current axis-aligned min-edge-spacing rule, derived layers beyond Glassworks's current bounds-based named `AND`/`OR`/`NOT` intersections/source unions/differences, and rule categories.

Glassworks has basic data-driven rules for min width, max width, min area, max area, bounds spacing, axis-aligned edge spacing, enclosure, forbidden overlap, and off-grid checks, deterministic editable JSON DRC deck export/import with bounds-based named `AND`/`OR`/`NOT` derived layers and derived min/max-width, min/max-area, min-spacing, min-edge-spacing, plus forbidden-overlap checks that overrides later full/cell/region runs and survives app-session save/load, explicit full-layout, current-cell, and selected rectangle/polygon-region DRC runs, plus active-marker annotation-layer output, but not a general KLayout-like DRC language or report ecosystem.

KLayout source: [DRC Basics](https://www.klayout.org/downloads/master/doc-qt5/manual/drc_basic.html), [DRC Layer Reference](https://klayout.org/downloads/master/doc-qt4/about/drc_ref_layer.html)

### LVS And Device Extraction

- Layout-to-netlist extraction beyond Glassworks's current connectivity-component plus simple MOS-like device netlist JSON, SPICE-style `.subckt` export, and connectivity-only SPICE schematic comparison, including richer device-aware extracted circuits.
- Device extraction beyond Glassworks's current diffusion/poly crossing MOS-like devices with side-label source/drain/gate and overlapping well/body-label bulk inference, isolated labeled-poly resistor-like devices, and labeled metal-overlap capacitor-like devices, including true channel-split diffusion connectivity, richer well/substrate handling, broader resistor/capacitor extraction, and parameter extraction.
- Schematic netlist import and comparison beyond Glassworks's current SPICE `.subckt` comparison for named nets, supported MOS/resistor/capacitor device signatures, and exact MOS `L`/`W` dimension strings when supplied by the schematic, including hierarchy, broader devices, richer parameters, and property-aware matching.
- LVS report and cross-reference browsing.
- Hierarchy-aware LVS workflows and netlist browser integration.

Glassworks has full-layout flattened-geometry connectivity extraction that is independent of current display visibility, open/short reports, deterministic extracted-netlist JSON exchange, simple diffusion/poly crossing MOS-like device recognition with side-label source/drain/gate and overlapping well/body-label bulk inference, isolated labeled-poly resistor-like devices, and labeled metal-overlap capacitor-like devices carried through netlist JSON and SPICE export, a SPICE-style `.subckt` export with generated names for unlabeled components and comments for opens/shorts, and SPICE `.subckt` schematic comparison for named nets plus supported MOS/resistor/capacitor signatures, exact MOS `L`/`W` dimension strings, selected-net device-terminal summaries, selected-net SPICE status, and compact missing/extra device details in the net browser, but not true channel-split diffusion connectivity, rich substrate tap modeling, broader resistor/capacitor recognition, richer parameter/property-aware device comparison, extracted circuits, hierarchy-aware LVS, or LVS-grade browsing.

KLayout source: [LVS Introduction](https://www.klayout.org/downloads/master/doc-qt5/manual/lvs_intro.html), [LVS Overview](https://www.klayout.org/downloads/master/doc-qt5/manual/lvs_overview.html), [LVS Device Extractors](https://www.klayout.org/downloads/master/doc-qt5/manual/lvs_device_extractors.html)

### Net Tracing Workflow

- Point-based single-net tracing beyond Glassworks's current basic Trace tool, including richer trace state controls.
- Trace-all-net workflows beyond Glassworks's current explicit Trace All action, extracted-component browser with labeled/unlabeled/device-connected/trace-history/shorted/open filters, and trace history seeding, including a full netlist browser.
- Trace path between two points beyond Glassworks's current route-point Trace Path connectivity check, including richer path visualization.
- Layer-stack editor for tracing configuration beyond Glassworks's current active built-in technology selector, stack browser, and per-link connectivity enable controls.
- Symbolic connectivity layers and boolean-derived connectivity layers.
- Save/load traced nets as L2N databases beyond Glassworks's current combined L2N database JSON exchange.
- Net tracing history beyond Glassworks's current searchable and clearable short trace history list, deterministic trace-state JSON round-trip, and selected/history/off trace-highlight modes.

Glassworks has full-layout flattened-geometry connectivity extraction that is independent of current display visibility, selected-net highlighting, a basic point Trace tool, a route-point Trace Path check, route-time same-net or same-component context for obstacle handling, an explicit Trace All action that seeds trace history and selects the largest component, selected/history/off trace-highlight modes, a searchable and clearable short trace history list with first-matching entry selection, deterministic extracted-netlist JSON, SPICE-style netlist export, connectivity-only SPICE schematic comparison, trace-state JSON export/import, combined L2N database JSON export/import, and a basic extracted-component net browser with labeled/unlabeled/device-connected/trace-history/SPICE-extra/shorted/open filters, select-first matching net selection, component issue rows, device count rows, and selected-net device-terminal summaries, but not KLayout's full trace-history/netlist-browser/L2N workflow.

KLayout source: [Net Tracing Feature](https://klayout.org/downloads/master/doc-qt4/manual/net_tracing.html), [DRC Netter Reference](https://www.klayout.org/downloads/master/doc-qt5/about/drc_ref_netter.html)

### Marker Browser And Report Databases

- Persistent multiple report databases per view beyond Glassworks's current bounded active-report list, app-session report-list restore, and deterministic report-database JSON exchange.
- Hierarchical marker categories beyond Glassworks's current fixed rule-family grouping.
- Marker directory and marker info panes beyond Glassworks's current rule-path directory rows, selected-marker info rows, active DRC report list, and marker list with category filters.
- Marker review workflows beyond Glassworks's current visited/important tags, hidden/waived state, preset notes, ownership presets, signoff presets, and preset key/value tags, including richer signoff metadata.
- Marker screenshot management and richer tagged-value editing UI beyond Glassworks's current selected-marker RGBA snapshot export, snapshot-tag filter, snapshot info rows, and preset/custom key/value marker tags.
- Open/reload/save report databases beyond Glassworks's current deterministic active-report and bounded report-database DRC JSON exchange, recent-file/reload integration, in-memory report selection, and active-marker annotation-layer geometry output.
- Import Calibre-style DRC/RVE marker formats beyond Glassworks's current text subset for polygon, rectangle, and edge marker geometry.

Glassworks has an active-report DRC marker browser with a bounded report history for recent full/cell/region runs and imported marker reports that survives app-session save/load, fixed rule-family category grouping, rule-path directory rows, selected-marker info/snapshot rows, active/hidden/waived/visited/important/noted/owned/signed-off/tagged/snapshot filters, click-to-focus and first-matching marker selection, selected-shape focus, automatic visited tagging, important tagging, preset review notes, owner presets, signoff presets, preset and custom key/value tags, selected-marker RGBA snapshot export with screenshot path/bounds/size tags carried by report JSON, deterministic active-report and bounded report-database DRC JSON export/import, Calibre/RVE-style text marker import for polygon/rectangle/edge geometry, active-marker annotation-layer geometry output, and persisted hide/waive/visit/important/note/owner/signoff/tags/clear state for stable DRC marker keys. It still does not have KLayout's generic persistent multi-RDB marker browser, user-defined hierarchical categories beyond the fixed rule-family set, embedded screenshot galleries, richer marker-info panes, full Calibre/RVE database compatibility, or broader signoff-style review metadata.

KLayout source: [Marker Browser](https://www.klayout.org/downloads/master/doc-qt5/manual/marker_browser.html)

### Layer Display And Layer Set Management

- Used/unused layer workflows beyond Glassworks's current layer-panel usage counts, row markers, All/Used/Empty row filter, active-layer isolation, invert visibility, inactive-empty-layer pruning, and basic visibility presets.
- Layer animations, bitmap/custom stipple patterns, and richer display styles beyond Glassworks's current persisted/rendered outline, hatched, cross-hatched, and built-in normal/dense/sparse stippled fill styles plus solid/dashed/dotted/dash-dot line styles.
- User-defined or technology-imported hierarchical layer trees beyond Glassworks's current fixed two-level All/FEOL/Routing/Text layer-panel group tree with FEOL and routing child filters.
- Multiple layer setups in tabs beyond Glassworks's current eight named tab-selectable layer setup slots.
- Load/save layer sets beyond Glassworks's current deterministic JSON exchange for eight layer setup slots that preserve setup names, visibility, fixed hierarchical layer group filter, layer-row usage filter, and per-layer hierarchy depth overrides.
- Richer hierarchy-depth setup workflows beyond Glassworks's current active-layer and current-layer-group options-backed depth override controls, including arbitrary saved group policies and bulk editing across imported group trees.
- Property selectors and view transformations beyond Glassworks's current property-bearing shape/cell/instance quick filters and browser-search property selectors for shape names, shape nets, and stored shape/cell/instance key/value metadata.

Glassworks has layer colors, visibility, order, purpose, technology-defined layers, used/unused usage markers, basic usage-driven visibility presets plus active-layer isolation, invert visibility, and undoable inactive-empty-layer pruning, options-backed fixed hierarchical layer filters for All/FEOL/Routing/Text groups, FEOL/Routing child groups, layer group tree summaries, and All/Used/Empty rows, persisted rendered fill/line display style controls with outline, hatched, cross-hatched, normal/dense/sparse stippled, dashed, dotted, and dash-dot 2D geometry effects, active-layer and current-layer-group hierarchy depth overrides persisted in options, eight named options-backed layer setup tabs that save visibility, layer group, row usage filter, and per-layer depth overrides, and deterministic JSON exchange for those slots, but not the full display/layer-set management model.

KLayout source: [KLayout Basics](https://www.klayout.org/downloads/master/doc-qt5/manual/basic.html)

### Browsers And Inspectors

- Shape browser beyond the current Glassworks current-view visible-occurrence browser with shared text search, configurable Summary/Geometry/Relations/All detail presets, quick active-layer/selected-shape/selected-net/property-bearing/shape-kind filters, `key=value` property-selector search, select-first matching occurrence action, sort controls, and property rows including stored shape key/value metadata: cross-layout browsing and richer object views.
- Instance browser beyond the current Glassworks current-cell and hierarchy-wide child-instance browser scopes with shared text search, configurable detail presets, named/property-bearing/array/identity/transformed quick filters, `key=value` property-selector search, select-first matching instance action, sort controls, and property rows including stored instance key/value metadata: richer filters and cross-layout browsing.
- Cell browser beyond the current flat all-cell list with shared text search, configurable detail presets, current/used/unused/library/property-bearing/empty/leaf/branch/parent/child quick filters, `key=value` property-selector search, select-first matching cell action, and collapsible hierarchy tree: richer hierarchy display modes and cross-layout browsing.
- Rich object property browsing across loaded layouts.
- Marker browser integration beyond the current DRC marker browser, including generic report databases and richer cell/object cross-probing.

Glassworks has selected-shape and selected-instance details plus standalone current-view shape, current-cell or hierarchy-wide instance, cell, DRC marker, and extracted-net browsers in the 2D inspector with shared text search, configurable browser detail presets, basic quick filters including selected, property-bearing, and shape-kind shapes with `key=value` property-selector search and select-first matching shape selection, named/property-bearing/array/identity/transformed instance views with `key=value` property-selector search and select-first matching instance selection, current/used/unused/library/property-bearing/empty/leaf/branch/parent/child cell views with `key=value` property-selector search and select-first matching cell viewing, DRC marker views with first-matching marker selection, and labeled/unlabeled/device-connected/trace-history/SPICE-extra/shorted/open net views with select-first matching net selection, sort controls, and property rows including stored shape, instance, and cell metadata key/value rows, but not KLayout's richer cross-layout browser suite with advanced object views.

KLayout source: [KLayout Basics](https://www.klayout.org/downloads/master/doc-qt5/manual/basic.html)

### Measurement, Images, And View Utilities

- Ruler workflows beyond Glassworks's current direct/horizontal/vertical/Manhattan measurement annotations, searchable measurement browser, active-layer and mode filters, listed-ruler aggregate summaries, mode/endpoint/delta/length/angle property rows, first-matching ruler selection, selected-ruler focus/deletion, and undoable visible-ruler cleanup.
- Add images into the view beyond Glassworks's persisted reference-image overlays, deterministic reference-image JSON exchange, visibility toggle, per-image focus/opacity/order/removal controls, and translucent image/placeholder overlay rendering.
- Landmarks to align images beyond Glassworks's current two-or-more-landmark axis-aligned reference-image alignment plus seed, clear, and selected-geometry fit controls.
- Bookmarked views and previous-view navigation beyond Glassworks's current eight-slot named/options-backed layout view bookmarks, deterministic bookmark JSON exchange, origin/bounds/selection focus, clear actions, and previous-view toggle.
- Full session save/restore beyond Glassworks's current deterministic app-session JSON that bundles workspace data, app options, active shell/view state, current 2D layout view, and recent exchange-path list, including richer multi-window session chrome.
- Screenshot export workflows beyond Glassworks's current File-menu deterministic RGBA and portable PPM snapshot export.

Glassworks has measurement annotations with direct/horizontal/vertical/Manhattan drawing modes, a searchable measurement browser, active-layer and mode filters, listed-ruler total-length and mode summaries, mode/endpoint/delta/length/angle property rows, first-matching ruler selection, selected-ruler focus/deletion, and undoable visible-ruler cleanup, persisted reference-image overlays with global/per-image visibility, per-image focus/opacity/order/removal controls, landmark-based axis-aligned alignment, landmark seed/fit/clear controls, snapshot tooling, File-menu deterministic RGBA and portable PPM screenshot export, File-menu app-session/layout/workspace JSON save/load, File-menu JSON/GDS/CIF/DXF/DEF import as child-cell, extra-top-cell, and flattened-merge workflows, File-menu recent entries and reload-most-recent for native session/layout/workspace/GDS/CIF/DXF/DEF/LEF/reference-image exchange paths, Origin/Layout Bounds/Selection focus actions, and eight named/options-backed layout view bookmark slots with deterministic JSON exchange, clear actions, and previous-view navigation, but not KLayout's full interactive ruler/image/session/view workflow.

KLayout source: [KLayout Basics](https://www.klayout.org/downloads/master/doc-qt5/manual/basic.html)

### Automation, Macros, And Plugins

- Ruby/Python scripting against the layout database.
- Macro Development IDE.
- Scriptable DRC/LVS/net extraction workflows.
- Plugin factories, toolbar integration, and custom UI extensions.
- Broader package/plugin ecosystem.

Glassworks has Rust internals and command-like UI actions, but not a user-facing scripting or plugin system.

KLayout source: [KLayout User Manual](https://www.klayout.org/downloads/master/doc-qt5/manual/index.html), [Application API](https://www.klayout.org/downloads/master/doc-qt4/programming/application_api.html)

## Lower-Priority Or Specialized Gaps

- Manufacturing/EDA-specific property propagation and richer property-aware selection beyond Glassworks's current browser property-selector search and select-first actions.
- Rich text handling and text-object options beyond Glassworks's current basic label geometry and direct Label tool.
- Advanced import/export compatibility controls for GDS variants.
- Gerber import preparation workflows.
- Multi-layout comparison workflows beyond Glassworks's current layout diff surface.
- Technology manager UI for editable libraries, layer properties, macros, net tracer settings, and connectivity stacks beyond Glassworks's current active built-in technology selector, stack browser, and per-link connectivity enable controls.
- External marker/database interchange for signoff review beyond Glassworks's current Calibre/RVE-style text marker subset.

## Suggested Roadmap Order

1. OASIS plus stronger GDS reader/writer options.
2. Complete hierarchy navigation: richer cell-management workflows.
3. Layer/shapewise boolean and sizing operations.
4. KLayout-style marker/report database import/export.
5. Point net tracing and full trace-all-net browser.
6. Scriptable DRC deck language or a deliberately smaller dataflow rule system.
7. Deeper dynamic PCells or persistent reusable libraries.
8. LVS/device extraction only after connectivity, derived layers, and report browsing are stronger.

## Notes

- Some KLayout features overlap with Glassworks concepts but are still listed because Glassworks's implementation is narrower. For example, Glassworks has DRC, but not scriptable DRC; has a flat hierarchy cell browser with current/used/unused/library/property-bearing/empty filters, collapsible hierarchy tree, parent/child context actions, selected-instance descend, view-only cell hide/show, box-only hierarchy view mode, numeric min/max hierarchy controls, shallow selected-instance variants, standalone current-cell duplication, and current-view shape/instance browsers, but not deeper KLayout-style variant policies; has a parameterized via-array library macro with stored generation metadata, selected guide-shape conversion, detach-to-static metadata clearing, and four options-backed preset slots with deterministic catalog exchange, but not dynamic editable PCells or full persistent layout libraries; has full-layout flattened-geometry connectivity, simple MOS-like, resistor-like, and capacitor-like device recognition, a basic point Trace tool, direct net-label placement, selected/history/off trace highlighting, a searchable and clearable short trace history list, deterministic extracted-netlist and trace-state JSON exchange, SPICE export and supported MOS/resistor/capacitor-signature comparison, and a basic extracted-component net browser with labeled/unlabeled/device-connected/trace-history/SPICE-extra/shorted/open filters, but not full L2N browsing or full device-aware LVS; has measurement annotation property rows, selected-ruler focus and visible-ruler cleanup, persisted reference-image overlays with visibility, focus, opacity, order, removal, and landmark alignment, deterministic app-session/layout/workspace JSON save/load/reload-most-recent, JSON/GDS/CIF/DXF/DEF import as child-cell, extra-top-cell, and flattened-merge workflows, options-backed recent exchange paths, and eight named/options-backed layout view bookmark slots with deterministic JSON exchange, origin/bounds/selection focus, clear actions, and previous-view toggle, but not full multi-view session restore.
- KLayout is a mature general layout viewer/editor. Glassworks's strongest differentiators are native/WASM Rust architecture, collaboration, fab-domain workflow modeling, and custom 3D/process views, which are outside this specific 2D gap list.

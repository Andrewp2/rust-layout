# Non-UI Foundation Audit

This tracks backend and domain work that can move independently while the UI layer evolves. The goal is to make the product harder to corrupt, easier to load on every target, and measurable under realistic demo data before more UI is layered on top.

## Current Slice

- Added a `WorkspaceDataset` validation pass before applying loaded workspace state. Invalid workspace bundles now fail closed instead of partially replacing app state.
- Covered workspace schema version, document schema/grid/top cell, shape layer references, cell shape layer references, cell instance targets, next ID counters, MES route/traveler links, process-flow recipe links, scheduler recipe links, yield summary links, wafer map die links, inventory lot/wafer/shape links, and notebook external-link warnings.
- Added JSON persistence round-trip coverage for blank and demo workspaces.
- Added validated atomic workspace writes so invalid bundles do not replace the last known-good workspace file.
- Added a web-safe path through the same validator by routing the built-in demo workspace through `apply_workspace_dataset`.
- Added a wasm crate regression path that validates the in-memory built-in demo workspace without filesystem access and exposes the same check to wasm JavaScript as `validateBuiltinDemoWorkspace`.
- Added a testable native demo-workspace loader that regenerates malformed or invalid-schema demo bundle files into a valid demo workspace.
- Added `ConnectivityReport::summary(max_issue_rows)` in `layout_model` so dense net extraction results have a stable compact shape: health, issue counts, displayed/omitted row counts, labeled component count, largest component size, and skipped reason.
- Added DRC stable issue keys and `DrcIssueStore` in the DRC crate so marker state can depend on backend issue identity instead of app-local row numbering.
- Added connectivity stable issue keys and `ConnectivityIssueStore` in `layout_model` so opens and shorts can be queried by backend identity and capped independently from raw extraction output.
- Added small golden fixtures for connectivity and DRC: one fixture asserts clean connected routing, a short, and an open in the same document; the other asserts each DRC rule family fires exactly once.
- Hardened connectivity issue stable keys so generated component IDs no longer affect short/open identity, with a regression proving keys survive recomputed component numbering.
- Added native DRC marker-state regression coverage proving hidden/waived state follows a stable violation key after recomputation changes row IDs.
- Routed native connectivity issue rows through `ConnectivityIssueStore` so net-panel rows carry backend stable keys before filtering/focus behavior.
- Added renderer-independent `RenderBatch3d` geometry validation for finite vertices, index ranges, triangle degeneracy, guide line indices, and rect-slab extents before native 3D scene upload.
- Added `import_gdsii_with_report` so GDS import reports generated fallback layers for unmapped layer/type pairs instead of requiring callers to infer them by scanning the imported document.
- Added `export_gdsii_with_report` so GDS export reports emitted structures/elements and records degenerate geometry or missing instance targets skipped during export.
- Added native 3D stack regression coverage for ordered/non-overlapping layer heights, contact/via touch points, and cached scene slab `z_range`s for overlapping layer fixtures.
- Added native 3D camera-axis regression coverage for finite orthonormal bases, finite view-projection matrices, and non-empty frustum queries on +X, -X, +Y, -Y, and steep top-down views.
- Added sync-server persisted-state schema rejection and atomic replacement for persisted collaboration state.
- Added workspace snapshot metadata to saved bundles: producer, producer version, workspace schema, document schema, feature flags, and migration history, with legacy missing-metadata snapshots accepted with a validation warning.
- Added a deterministic document performance-budget report for Loro seeding, DRC, connectivity, and 3D shape-count caps.
- Added a source-controlled demo workload performance baseline JSON fixture that records budgets for demo load, DRC, connectivity, layout diff, reticle checks, and 3D mesh build while the test reports input size and elapsed time.
- Added `FabMesData::validate()` in `layout_model` so MES route, lot, wafer, traveler, active-run, signoff, hold, and terminal-state invariants are owned by the domain model and reused by workspace validation.
- Added `Inventory::validate_with_context()` in `layout_model` so material lots own stock/date/usage integrity checks and validate lot, wafer, layout-shape, and tool-run traceability links through a shared reference context.
- Added `DispatchSchedule::validate_with_context()` in `layout_model` so scheduler tools, lots, assignments, maintenance windows, recipe references, and MES lot planning warnings are checked before workspace state is accepted.
- Added `WaferMap::validate_with_context()` in `layout_model` so metrology geometry, die lists, measurements, defects, annotations, lot links, and wafer links are validated by the metrology domain before workspace state is accepted.
- Added `LabNotebook::validate_with_context()` in `layout_model` so notebook entry IDs, tags, required link fields, images, metrology references, and external lot/wafer/recipe/metrology warnings are owned by the notebook domain.
- Added `YieldAnalysis::validate()` in `layout_model` so yield recipes, lots, wafers, test results, process measurements, summaries, comparisons, and correlations are checked by the yield domain before workspace state is accepted.
- Added `ProcessFlowModel::validate_with_context()` in `layout_model` so process-flow structural findings, MES route references, and recipe bindings are validated by the process-flow domain before workspace state is accepted.
- Added domain-owned validation for cleanroom environment, maintenance, and safety models, then wired those findings into workspace acceptance so broken sensor, alarm, tool, downtime, interlock, and incident references fail before persistence or apply.
- Added domain-owned validation for DOE experiment plans and R2R process-control models, then wired those findings into workspace acceptance so corrupt run matrices, response values, loop limits, trend points, actions, and audit links fail before persistence or apply.
- Added domain-owned validation for genealogy and equipment models, then wired those findings into workspace acceptance so corrupt wafer ancestry, material usage, sequence counters, tool recipes, run state, alarms, sensors, and logs fail before persistence or apply.
- Moved workspace bundle schema, snapshot metadata, blank/demo constructors, built-in demo reporting, and cross-domain validation into `layout_model::workspace` so native and wasm share the same data contract while native keeps filesystem I/O and atomic writes.
- Added `RecipeCatalog::validate()` and wired it into workspace acceptance so duplicate recipe versions, route IDs, tool-run IDs, bad recipe bindings, and bad route/tool-run references are caught by the shared model layer.
- Added shared workspace JSON load/migration through `WorkspaceDataset::from_json_str`, including schema-0 and missing-schema legacy migration tests plus native read-path coverage.
- Added web startup query parser regression coverage for `?workspace=demo` / `?demo=true` and native startup-option coverage proving demo startup loads the built-in workspace without filesystem access.
- Repaired `scripts/render_e2e.py` so its screenshot smoke explicitly loads `?workspace=demo` for demo/layout/3D/options cases, and verified the Trunk web entry point with both the render smoke matrix and a local `?workspace=demo&view=metrology` headless Chrome smoke.
- Added GDS import coverage for unknown TEXT layer/texttype pairs so generated import-layer reporting now covers both geometry and label/text mappings.
- Added startup-path 3D geometry coverage for demo workspace, hierarchy scene, and stress scene generation, including cached batch reuse and renderer-independent geometry validation.
- Added caller-visible GDS export warnings for lossy-but-emitted data: fallback layer mappings, missing-layer numeric fallbacks, unsupported instance transforms, and shape metadata that cannot round-trip through GDS.
- Added hierarchy and stress performance baseline fixtures beside the demo workload fixture, including deterministic budget/skip checks and 3D mesh-build ceilings without treating elapsed wall-clock time as a correctness gate.
- Added a source-controlled observed-performance report example, serializer coverage, and an opt-in `FABRICAD_PERFORMANCE_REPORT_DIR` artifact writer so elapsed samples can be persisted/reported as diagnostics without becoming flaky pass/fail thresholds.
- Added native and sync-server atomic-write fault injection proving an interrupted temp-file write preserves the previous save file and cleans the temporary file.
- Hardened workspace document validation so `next_shape_id` covers child-cell shapes and `next_cell_id` / `next_instance_id` must stay above existing hierarchy IDs.
- Added persistent connectivity issue state beside DRC marker state, so stable net short/open keys can carry hidden/waived/note state through document JSON, workspace bundles, and recomputed component IDs.
- Hardened native 3D slab triangulation so colinear polygons emit no mesh and duplicate-vertex polygons skip degenerate triangles/faces while preserving valid geometry.
- Promoted the DRC and connectivity golden scenarios into source-controlled JSON fixtures under `fixtures/quality/`, with tests loading fixture geometry and expected summaries instead of embedding every shape in Rust.
- Added optional technology-owned 3D stack metadata (`z_base` / `z_thickness`) to built-in process layers and made native 3D rendering prefer those ranges with legacy hard-coded fallback.
- Added a source-controlled GDS round-trip fixture under `fixtures/import_export/` that checks export/import reports plus top geometry, child-cell geometry, labels, paths, and AREF instance arrays.
- Hardened workspace JSON loading with a schema preflight so future workspace bundles fail with an explicit unsupported-schema error before attempting to deserialize a payload that may no longer match the current model.
- Added an app-level quality-fixture validator and wasm-accessible `validateQualityFixtures` entry point so DRC/connectivity fixtures are checked outside the model crates and without filesystem access.
- Added shared JSON-loader and native demo-loader regressions for syntactically valid but cross-domain-invalid workspace bundles, proving bad demo files regenerate instead of partially loading corrupt recipe references.
- Added import-side GDS skipped-element reporting for degenerate BOUNDARY/PATH elements so invalid incoming geometry is visible to callers instead of silently disappearing or becoming unusable shapes.
- Added MES-backed genealogy validation context and wired workspace validation through it, so genealogy routes, process steps, recipes, and tool references are checked against authoritative MES route data.
- Promoted the future workspace-schema preflight case into a source-controlled persistence fixture and extended the connectivity golden fixture with stable issue keys plus persisted hidden/waived/note state examples.
- Added a source-controlled custom technology stack fixture that validates user-style layer stack metadata, connectivity, DRC references, GDS mapping, and document layer construction outside the built-in technology files.
- Added a Foundation Checks workflow that runs native performance-budget fixtures with `FABRICAD_PERFORMANCE_REPORT_DIR` and uploads the observed JSON reports as a CI artifact.
- Added native imported-GDS 3D coverage: a hierarchy/path layout is exported, imported, loaded into the app, and validated through the same cached 3D scene geometry path as user imports.
- Updated native GDS import status to report structures, elements, generated layers, and skipped import elements so model-level import loss reports reach the app path.
- Updated native GDS export status to summarize warning classes (fallback mappings, missing layers, unsupported transforms, and non-round-trippable metadata) instead of exposing only an opaque warning count.
- Added shared document operations for DRC marker state and connectivity issue state, then routed native waive/hide/clear review actions through undoable CRDT-backed history without forcing layout index or connectivity rebuilds.
- Added a source-controlled schema-0 workspace fixture that exercises legacy migration, metadata regeneration, persisted review states, current-schema reserialization, and the native filesystem read path.
- Hardened GDS import so missing SREF/AREF targets no longer abort the entire file; the importer now keeps valid geometry and reports broken instance references as skipped import elements.
- Added a hierarchy-fanout performance fixture and made 3D budget checks use flattened rendered-shape estimates, so compact documents with huge instance arrays are capped by renderer-facing workload instead of top-level shape count.
- Added an imported-layout performance fixture that covers GDS export/import reports, generated/skipped import counts, diffing an imported document, and validating the imported layout through the 3D mesh path.
- Added equipment-backed maintenance validation so workspace checks now catch maintenance tools missing from the simulator and qualification recipes that are not available on the referenced equipment tool.
- Hardened MES validation for empty route/lot/traveler/wafer/tool IDs and completed-step history, including recipe and eligible-tool checks against the route step definition.
- Added yield-backed process-control validation so R2R loops, trend points, actions, recipe bindings, measurement IDs, measurement names, metrology steps, and source runs are checked against the workspace yield-analysis context.
- Added equipment-backed safety validation so tool-linked safety sensors, tool interlocks, and incidents must reference actual equipment tools before workspace state is accepted.
- Added workspace-backed DOE validation so experiment route/step, baseline recipe, recipe-parameter factors, metrology response steps, run lot/wafer assignments, run recipes, and assigned tools are checked against MES, recipe catalog, yield, and equipment context.
- Added MES/catalog-backed equipment validation so selected and running equipment recipes now verify lot, wafer, route-step, and catalog-version references where those IDs are authoritative, while equipment-local recipe programs remain explicit warnings instead of false hard failures.
- Added MES route-step context to scheduler validation so scheduled lots that exist in MES must point at an actual route step with the matching required recipe and tool class, and scheduler tools now warn when they are not listed as eligible on any MES route step.
- Hardened scheduler assignment validation so persisted dispatch rows must still match their referenced lot priority, due time, ready time, process duration, derived wait/tardy values, compatible tool, and maintenance-window availability instead of being accepted as stale copied metadata.
- Added GDS import reporting for known unsupported element types such as BOX/NODE/TEXTNODE, so valid surrounding geometry still imports and unsupported elements show up in skipped-element reports with layer/type details.
- Added workspace metadata/document-schema preflight so bundles with future metadata or document payload schemas are rejected before deserialization instead of being misclassified as schema-0 legacy snapshots or failing with generic missing-field parse errors.
- Added 3D mesh-validation samples to demo, hierarchy, stress, and imported-layout performance fixtures so generated and imported scene reports budget validated triangle/slab/guide primitive counts instead of relying only on ad hoc assertions.
- Added caller-visible GDS export warnings for ambiguous layer mappings, so document layers that share the same exported geometry or text GDS pair are reported before they silently merge in downstream tools.
- Added caller-visible GDS import warnings for split incoming layer mappings, so one GDS layer number fanning out into multiple Fabricad layers is reported instead of only being implied by generated-layer rows.
- Promoted the split incoming GDS layer warning case into a source-controlled import/export fixture so future external GDS loss-report cases can extend the same path.
- Hardened workspace metadata validation so current-schema snapshots declaring unsupported feature flags fail validation before app state is applied, while duplicate supported flags remain warning-only.
- Hardened document layer validation inside workspace acceptance so corrupt layer names and out-of-range/non-finite RGBA channels fail before load, while duplicate layer names are surfaced as warnings.
- Hardened persisted DRC/connectivity review-state validation so default no-op states and empty notes are rejected instead of polluting marker/issue review counts.
- Added workspace validation as an explicit demo performance-budget sample so CI observation artifacts track validation elapsed time and assert zero validation errors alongside DRC/connectivity/3D budgets.
- Added a source-controlled unsupported workspace feature-flag fixture and native read-path coverage so current-schema bundles requiring future capabilities fail validation with a specific compatibility error.
- Added shared native/wasm persistence fixture validation so legacy migration, future schema rejection, and unsupported feature-flag rejection are checked without native filesystem access.
- Added a Foundation Checks web render-smoke CI job that installs wasm/Trunk/Chrome dependencies, runs `wasm_app` regressions, exercises the Trunk demo render matrix, and uploads screenshot/log artifacts.
- Promoted unsupported GDS BOX/NODE/TEXTNODE import coverage into a source-controlled fixture that proves valid neighboring geometry still imports while unsupported elements are reported as skipped.
- Promoted degenerate incoming GDS BOUNDARY/PATH loss-report coverage into a source-controlled fixture so invalid external geometry remains visible to callers without importing unusable shapes.
- Promoted missing SREF/AREF target coverage into a source-controlled GDS import fixture so broken hierarchy references are reported as skipped instances while valid neighboring geometry still loads.
- Extended the app-level and wasm quality-fixture validator to assert connectivity stable issue keys, DATA component shape coverage, and fixture review-state records instead of checking only aggregate short/open counts.
- Hardened workspace snapshot metadata validation so empty migration-history entries fail load/read while repeated entries are surfaced as warnings instead of silently polluting version history.
- Added imported-layout performance coverage for technology-owned 3D stack ranges, so imported geometry reports validated z-range coverage separately from generic mesh primitive validation.
- Hardened workspace snapshot metadata validation so current snapshots with an empty producer version fail shared JSON loading and native filesystem reads instead of becoming unauditable save bundles.
- Hardened inventory validation so material usage quantities must be compatible with the material lot stock unit family, allowing expected mL/L and g/kg conversions while rejecting incompatible ledgers such as wafer counts against liquid photoresist stock.
- Hardened equipment validation so alarms, sensor samples, and tool log entries warn when their timestamps are after simulator time, matching the existing run timestamp checks and surfacing future-dated operational telemetry before it enters workspace state.
- Hardened equipment run identity validation so an active run ID cannot also appear in the recent-run log, preventing duplicated in-flight/completed run records from double-counting the same tool activity.
- Hardened maintenance validation so completed historical records warn when calibration, qualification, downtime, or spare-part dates are after the model's `today`, while leaving future schedule due dates valid.
- Hardened maintenance `FabDate` validation to enforce real calendar month lengths and leap-year rules instead of accepting every day 1-31 for every month.
- Hardened inventory `YYYYMMDD` date parsing to require four-digit fab years plus real calendar month lengths and leap-year rules, so impossible expiration, received, and usage dates fail validation.
- Hardened cleanroom environment validation so duplicate readings for the same sensor and timestamp fail validation before they can destabilize latest-reading summaries or generated alarm identities.
- Hardened safety interlock validation so a tool interlock cannot list the same required sensor more than once, preventing ambiguous safety requirements even when lockout reason display later deduplicates them.
- Hardened process-control action validation so one R2R control action cannot carry duplicate adjustments for the same recipe parameter, avoiding ambiguous proposed recipe changes.
- Hardened process-flow validation so persisted routes reject empty stable IDs, duplicate edge IDs, and route version 0 before those records can be exported into MES route definitions.
- Hardened process-flow version-history validation so duplicate/zero version records, stale current node/edge counts, and missing current-version records are caught by model and workspace validation before persisted audit trails become misleading.
- Hardened genealogy validation so lot ancestry cannot self-reference or repeat sources, and wafer parent links must point at a real parent wafer rather than only an existing parent lot.
- Hardened yield-analysis validation so duplicate lot/wafer summaries, misplaced summary scope, duplicate/self lot comparisons, stale yield deltas, and inconsistent failure-mode deltas fail before yield dashboards consume arbitrary first-match records.
- Hardened metrology wafer-map validation so each die can carry at most one measurement per measurement kind, preventing `measurement_for` and summary consumers from silently choosing one of multiple conflicting records.
- Hardened recipe-catalog validation so route step sequence 0, tool-run route version 0, tool-class mismatches against referenced process steps, and impossible tool-run timestamp/state combinations fail or warn before recipe usage history becomes unreliable.
- Hardened lab-notebook validation so updated-before-created entries and metrology wafer links without lot context fail before cross-domain notebook evidence becomes ambiguous.
- Hardened MES validation so route step sequence 0, incomplete wafer scrap/rework reasons, signoff-step completion without a signer, stray non-signoff signer metadata, impossible reworked wafer counts, and empty active-run operators are surfaced before traveler history drives scheduling or genealogy.
- Hardened inventory usage-ledger validation so incomplete receiving metadata, impossible usage-before-receipt dates, blank traceability IDs, zero layout-shape references, and repeated usage links are surfaced before material history feeds lot genealogy or fab dispatch decisions.
- Added equipment-runtime context to scheduler validation so dispatch tools and assignments warn when persisted planning availability or compatible recipes disagree with the live equipment simulator state.
- Hardened cleanroom environment validation so persisted alarms must match their source reading, threshold-derived severity, and active/latest state, while malformed correlation labels and duplicate sensor links are surfaced before facility excursions feed yield or process-hold decisions.
- Hardened safety validation so stale sensor severity/state, malformed incident and audit timestamps, duplicate alarm routes, empty interlock sensor IDs, duplicate incident route targets, and incidents below linked sensor severity are surfaced before safety state feeds equipment lockout or dispatch decisions.
- Hardened maintenance validation so zero PM intervals, empty or repeated checklist entries, incomplete calibration/qualification metadata, multiple open downtime records for one tool, ownerless downtime, and blank spare-part metadata are surfaced before release status or dispatch consumes maintenance history.
- Hardened process-control validation so stale R2R actions, malformed proposal/audit timestamps, duplicate source-run actions, audit/action loop mismatches, missing audit transitions, and latest-audit/action-state disagreements are surfaced before recipe-control history drives future process changes.
- Added SPC/FDC monitor self-validation so duplicate or malformed chart/trace IDs, stale rule/trace violation links, impossible control/sensor limits, mismatched alarm summaries, and finding-count drift are caught before generated process-monitor state is consumed by analysis views or future persistence.
- Hardened notebook context validation so external lot/wafer evidence remains warning-only, but metrology links that pair two known internal objects from different lots now fail as contradictory structured evidence.
- Hardened GDS import reports so zero or negative PATH widths that are salvaged into positive Fabricad path widths now emit caller-visible normalization warnings instead of only tracing an internal parser warning.
- Added connectivity report self-validation so stale shape/component mappings, bad component summaries, invalid short/open references, duplicate issue keys, and malformed skipped reports are caught before quality fixtures or review-state lookup consume them.
- Added DRC issue-store self-validation so stale lookup indexes, duplicate stable issue keys, malformed violations, bad shape references, and impossible metric values are caught before quality fixtures or marker review-state lookup consume them.
- Hardened GDS export loss reporting so semantic Fabricad shape kinds that cannot round-trip through plain GDS, such as vias and measurements, now emit explicit caller-visible warnings and native status summaries.
- Hardened GDS import reports so invalid AREF `COLROW` dimensions normalized to at least one row/column now emit caller-visible warnings and native import status counts instead of only internal parser logs.
- Hardened GDS export so oversized instance arrays no longer wrap row/column counts into invalid AREF records; arrays exceeding the GDS `COLROW` range are skipped with an explicit export-report reason.
- Added cross-section process validation so invalid process widths, column counts, substrate thicknesses, material catalogs, step material references, no-op deposit/etch values, and malformed mask openings are surfaced by the fab-domain model instead of being silently normalized by simulation.
- Hardened layout diff so summaries, bounds, and change rows compare visible flattened hierarchy occurrences instead of only top-level shapes, catching moved instances and child-cell geometry changes.
- Hardened reticle prep validation so empty reticle/layout identifiers, impossible printable clearances, duplicate exposure blocks, duplicate block layers, empty block names, and non-finite focus offsets fail before large mask issue reports or handoff summaries consume corrupt prep data.
- Hardened mask-check issue budgeting so retained issue rows and omitted issue counts are tested directly, and native performance fixtures budget total reticle issues rather than only the capped retained rows.
- Hardened GDS import reporting for duplicate structure names, warning callers about ambiguous `STRNAME` definitions and avoiding orphan duplicate cells while importing duplicate definitions into one Fabricad cell.
- Hardened GDS parsing so unterminated structures, unterminated elements, overlapping element starts, and missing `ENDLIB` records fail with explicit parser errors instead of silently dropping partial import data.
- Hardened workspace schema preflight so wrong-type and negative schema fields fail with explicit compatibility errors before serde deserialization, preserving clear load/regeneration diagnostics for malformed save bundles and the shared native/wasm persistence fixture validator.
- Hardened SPC/FDC monitor identity generation so one process metric measured in multiple units gets distinct stable chart IDs, and workspace validation now regenerates and validates the derived SPC/FDC monitor before accepting a bundle.
- Moved the cross-section process model into shared workspace persistence with a backward-compatible default for older bundles, native save/load wiring, and workspace validation that rejects corrupt cross-section material references before app state is applied.
- Hardened GDS import loss reporting so coordinates that overflow Fabricad coordinate range during DBU scaling now emit caller-visible clamped-coordinate warnings and native import status summaries instead of only internal trace logs.
- Hardened scheduler validation so persisted dispatch schedules reject duplicate lot assignments and overlapping assignments on the same tool instead of accepting individually valid rows that cannot coexist on the fab floor.
- Hardened scheduler/equipment validation so persisted dispatch assignments warn when they target an equipment tool whose runtime active run is already processing a different lot.
- Hardened cleanroom environment validation so facility events reject duplicate timestamp/zone/title identities and warn on empty zone/detail metadata before environment history is consumed by workflow or yield context.
- Hardened metrology validation so persisted measurements whose numeric value is outside a hard process spec cannot retain a stale pass/outlier status before wafer-map and yield summaries consume them.
- Hardened GDS export loss reporting so zero or negative Fabricad path widths are exported with a positive GDS PATH width and reported in both model-level export warnings and native status summaries.
- Added document-aware DRC rule-deck validation for missing layer references, non-positive rule distances, duplicate enclosure/overlap rules, and empty overlap names, then wired the quality-fixture validator through it before issue-store generation.
- Hardened workspace metadata preflight so malformed `feature_flags` and `migration_history` array fields fail with explicit compatibility errors before serde payload deserialization, with shared native/wasm persistence-fixture coverage.
- Hardened technology validation so process files reject out-of-range layer colors and overlapping 3D stack ranges before those definitions feed document creation, DRC assumptions, import/export mapping, or 3D slab rendering.
- Hardened technology application so layers with implicit IDs skip IDs reserved by later explicit layers instead of silently overwriting layer records during document construction.
- Hardened GDS import reporting so unsupported or malformed `UNITS` records normalized during import now produce caller-visible warnings and native status summaries instead of only trace logs.
- Hardened technology DRC validation so duplicate min-width or min-spacing rules for the same normalized layer reference fail before `RuleDeck` construction silently collapses them.

## Remaining Work By Area

1. Data model hardening
   - Continue closing stable ID gaps inside fab domain models, especially where references are historical, external, or represented only as free text.
   - Add explicit migration transforms for any future workspace schema increments before incrementing `WORKSPACE_DATASET_SCHEMA_VERSION`.

2. Web and demo loading
   - Extend malformed demo-bundle fixtures to unknown future domain schema once individual domain models grow independent schema versions.

3. DRC and connectivity quality
   - Extend quality fixture coverage into future UI/visual smoke harnesses once screenshot automation consumes non-model fixtures directly.
   - Add richer review-state flows once marker notes, issue ownership, or batch triage become first-class product concepts.

4. 3D geometry correctness
   - Extend technology-owned stack metadata into any future process-stack editor and imported-layout fixture reports.
   - Extend mesh validation into any future generated scene or user-imported workload fixtures as those variants are added.

5. Import and export
   - Expand GDS/OASIS-like fixture coverage around richer unsupported-element cases and loss reports.
   - Extend import reports to any additional unsupported element families discovered in external GDS files.
   - Extend export warning details into future OASIS-like export paths.

6. Performance budgets
   - Extend fixture coverage to future pathological imported layouts and hierarchy fanout variants as they are discovered.

7. Fab domain model
   - Extend equipment validation into scheduler dispatch context once planning assignments and runtime runs share a first-class runtime handoff model.
   - Continue aligning synthetic equipment programs with catalog recipes where they are intended to be authoritative; keep equipment-local runtime programs explicit as warnings.
   - Define which IDs are internal, external, historical, or planning-only so validation can distinguish errors from warnings.

8. Persistence and versioning
   - Extend source-controlled migration fixtures beyond schema-0 as additional workspace schemas are introduced.
   - Keep preflight fixtures for both top-level and metadata schema fields whenever workspace version handling changes.
   - Add interrupted-save fault injection to any future persistence path added outside native and sync-server state.

## Verification Strategy

- Unit tests should cover small, deterministic domain invariants.
- Fixture tests should cover realistic cross-domain workspaces and broken references.
- Wasm checks should prove no native-only filesystem assumptions leak into web demo loading.
- Performance tests should report both absolute elapsed time and input size so failures are diagnosable.

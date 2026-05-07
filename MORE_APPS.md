Think of it as a small fab software suite, not a pile of unrelated apps. The strongest portfolio version would be a coherent system where the mask editor, wafer tracking, recipes, equipment logs, metrology, and yield analysis all share one data model.

Atomic Semi’s role is not just asking for a CAD toy. It emphasizes tools that connect software to physical manufacturing, including interactive editors, Rust/WASM, real-time collaboration, computational geometry, routing, GPU visualization, state synchronization, serialization, schema evolution, and performance optimization. 

## The best framing: build a mini FabOS

Call the whole portfolio suite something like:

```text
ReticleStitch FabOS
OpenFab Studio
ForgeFab
MiniFab
WaferWorks
FabStack
```

The individual apps could share:

```text
Rust backend
WebAssembly frontend modules
WebGPU visualization
WebSocket event sync
Versioned schemas
Operation log
Postgres or SQLite storage
Object storage for large files
Common wafer, lot, recipe, run, and measurement models
```

The key idea: every app knows about the same physical objects.

```text
Design → Mask → Process Route → Lot → Wafer → Tool Run → Sensor Data → Metrology → Yield → Process Change
```

That data chain is more impressive than building ten disconnected demos.

## 1. Mask-layout editor

This is still the centerpiece.

It handles:

```text
Layered polygon editing
Reticle or mask geometry
Design-rule checks
Routing
Annotation
Measurement
Cell hierarchy
GDS-like export
Real-time collaboration
```

This is the most directly aligned app for the role because it demonstrates low-latency CAD-like editing, geometry algorithms, routing, GPU rendering, and collaborative state sync. 

Make this the flagship.

## 2. Process traveler / MES-lite

A Manufacturing Execution System, or MES, tracks what is being made, where it is, what step it is on, and what is allowed to happen next. ISA-95 describes the interface between enterprise systems and manufacturing control systems, and it is often used as a framework for MES and manufacturing operations systems. ([isa.org][1])

Your simplified app:

```text
Wafer lots
Individual wafers
Process routes
Step-by-step traveler
Tool eligibility
Recipe binding
Operator signoff
Hold and release
Scrap and rework
Audit trail
```

Example workflow:

```text
Lot L-00042
  Wafer 01 through Wafer 25
  Current step: lithography expose
  Required recipe: LITHO_POLY_001
  Required tool class: mask aligner
  Status: waiting for operator
```

This would make the suite feel like software for a real lab instead of only a design tool.

Good MVP features:

```text
Create lot
Assign wafers
Define process route
Move lot through steps
Record operator, timestamp, tool, and recipe
Prevent invalid step transitions
Show WIP board
```

Strong advanced features:

```text
Branching process flows
Rework loops
Lot genealogy
Wafer-level state
Electronic traveler export
Schema migrations for process-route versions
```

## 3. Recipe manager

A recipe is the controlled set of parameters used to run a tool.

Examples:

```text
Spin coater:
  rpm: 4000
  duration: 45s
  acceleration: 1000 rpm/s

Plasma etcher:
  gas: CF4
  pressure: 80 mTorr
  RF power: 150 W
  duration: 60s

Furnace:
  temperature: 950 C
  atmosphere: nitrogen
  duration: 30 min
```

Your app:

```text
Versioned recipe editor
Parameter validation
Recipe approval workflow
Recipe diff viewer
Recipe dependency graph
Recipe attached to process traveler steps
```

This is a great software-engineering project because it involves versioning, migrations, validation, permissioning, and auditability.

Make it feel like GitHub for fab recipes:

```text
Recipe POLY_ETCH_003
  v1: baseline
  v2: reduced etch time
  v3: changed pressure
  approved by: process engineer
  used by: lots L-00110, L-00111, L-00114
```

Portfolio angle:

```text
I built a versioned recipe system with typed parameters, validation rules, diffing, approval state, and links to wafer-run history.
```

## 4. Equipment control simulator

Real fabs need software that talks to equipment. In semiconductor manufacturing, SECS/GEM is a common equipment-to-host communication standard used for equipment automation, status reporting, alarms, events, data collection, and remote commands. ([SEMI][2])

Do not try to implement all of SECS/GEM. Build a simplified equipment protocol and simulator inspired by the real problem.

Your app:

```text
Virtual tools
Tool state machines
Remote start and stop
Alarm events
Recipe selection
Run logs
Sensor streams
Host commands
```

Example tools:

```text
Spin coater simulator
Hot plate simulator
Mask aligner simulator
Etcher simulator
Microscope simulator
Probe station simulator
```

Example state machine:

```text
Offline
Online idle
Recipe loaded
Running
Paused
Completed
Alarm
Maintenance
```

This would be extremely relevant because it shows real-time systems, networking, data synchronization, state machines, and physical-process modeling.

Good MVP:

```text
A Rust equipment simulator exposes WebSocket or TCP messages.
The MES sends a recipe.
The tool runs a fake process.
The tool streams sensor values.
The MES records the run.
Alarms interrupt the process.
```

Advanced:

```text
SECS/GEM-inspired message model
Equipment digital twin
Replayable run logs
Fault injection
Protocol trace viewer
Latency and throughput metrics
```

## 5. Tool dashboard / fab control room

This is the visual monitoring app.

It shows:

```text
Which tools are online
Which tools are idle
Which tools are running
Which lots are loaded
Current recipe
Current sensor values
Active alarms
Utilization
Downtime
Recent runs
```

A real fab has many tools, and operators need situational awareness. Your version could look like an air-traffic-control dashboard for lab equipment.

Views:

```text
Tool grid
Timeline view
Alarm feed
Run history
Sensor sparkline panel
Maintenance state
Lot currently loaded
```

This app pairs naturally with the equipment simulator.

Good portfolio detail:

```text
The dashboard receives live equipment events over WebSocket and renders state changes without polling.
```

## 6. Metrology and inspection viewer

Metrology is measurement. Inspection is finding defects or visual anomalies.

This app would ingest and visualize data from:

```text
Microscopes
Profilometers
Ellipsometers
Four-point probes
Film-thickness tools
Overlay measurements
Critical dimension measurements
Wafer maps
```

KLA describes semiconductor software for monitoring product quality, inspection, and metrology information, including wafer-fabrication excursion alerts and reports. ([KLA][3])

Your app:

```text
Upload microscope images
Attach images to wafer, lot, step, and tool run
Annotate defects
Measure distances on images
Compare before and after process steps
Render wafer maps
Link measurements to process recipes
```

This would map well to Atomic Semi’s interest in visualization, annotation, measurement, and tools for process engineers. 

Great MVP:

```text
Upload microscope image
Calibrate pixels to microns
Draw ruler annotations
Mark defects
Associate image with wafer W-00042, step POLY_LITHO
```

Advanced:

```text
Image tiling for huge microscope scans
GPU zoom and pan
Defect clustering
Before and after overlay
Measurement uncertainty
Collaborative annotation
```

## 7. Wafer map viewer

A wafer map shows spatial data across a wafer.

Examples:

```text
Die pass or fail
Film thickness
Sheet resistance
Critical dimension
Defect count
Overlay error
Etch rate
Probe-test result
```

This is one of the best visualization projects after the mask-layout editor.

Your app:

```text
Circular wafer view
Die grid
Color-coded measurements
Zoom into dies
Compare wafers
Compare lots
Histograms
Spatial heatmaps
Outlier detection
```

Example:

```text
Wafer W-00042
  center thickness: 103 nm
  edge thickness: 95 nm
  mean: 99.2 nm
  sigma: 2.8 nm
  failed dies: 12
```

Advanced:

```text
GPU-rendered wafer maps
Millions of measurement points
Brush and filter
Cross-section plots
Animated process drift over time
```

This is very aligned with high-performance visualization and large scientific datasets. 

## 8. SPC dashboard

SPC means Statistical Process Control. It tracks whether a process is stable over time. SEMI describes SPC as a central quality-management tool for predictable process quality. ([SEMI][4])

Your app:

```text
Control charts
Run charts
Histograms
Cp and Cpk
Outlier detection
Rule-based alarms
Process drift detection
Lot-to-lot comparison
```

Example charts:

```text
Metal1 sheet resistance over time
Photoresist thickness by lot
Etch depth by tool
Alignment error by operator
Defect count by recipe version
```

MVP:

```text
Load measurements
Plot control chart
Compute mean and control limits
Flag out-of-control points
Link each point back to wafer, lot, recipe, and tool run
```

Advanced:

```text
Western Electric style rules
Automatic excursion detection
Root-cause drilldown
Tool-to-tool matching
Recipe version comparison
```

This app gives you a data-analysis story, not just UI and graphics.

## 9. Fault detection and classification app

Fault Detection and Classification, or FDC, monitors equipment traces to detect abnormal runs. Sources describing semiconductor advanced process control often treat FDC as a way to detect abnormal equipment state, trigger alarms or halts, and classify causes. ([AIChE][5])

Your app:

```text
Ingest sensor traces from equipment simulator
Learn normal ranges
Detect abnormal runs
Classify likely fault cause
Show trace overlays
Trigger alarm events
Attach fault to lot and wafer
```

Example signals:

```text
Chamber pressure
RF power
Temperature
Motor current
Gas flow
Vacuum level
Spin speed
Exposure dose
```

Example fault:

```text
Run R-00921
  Tool: plasma etcher
  Recipe: POLY_ETCH_003
  Fault: pressure instability
  Severity: high
  Likely impact: etch nonuniformity
  Affected wafers: W-00042, W-00043
```

MVP:

```text
Generate synthetic traces
Compare trace to expected envelope
Flag excursions
Visualize where the run diverged
```

Advanced:

```text
Dynamic time warping between runs
PCA or clustering
Fault signatures
Operator-readable fault reports
```

This is a strong complement to the equipment simulator.

## 10. Run-to-run process control app

Run-to-run control adjusts future recipes based on previous measurements. In semiconductor process-control discussions, run-to-run control is commonly described as using post-process metrology and feed-forward data to adjust process recipes for later runs. ([AIChE][5])

Your app:

```text
Input: target film thickness
Input: measured film thickness from previous wafer
Output: adjusted recipe parameter
```

Example:

```text
Target oxide thickness: 100 nm
Measured oxide thickness: 96 nm
Model says: increase deposition time by 4.2 seconds
Proposed recipe override for next run: +4.2s
```

MVP:

```text
Simple linear process model
Manual approval for recipe adjustment
History of recommendations
Graph of target vs measured
```

Advanced:

```text
EWMA controller
Per-tool model calibration
Confidence intervals
Automatic hold if model uncertainty is high
```

This is a great way to show that your suite closes the loop:

```text
Run tool → measure wafer → analyze drift → adjust next run
```

## 11. Process-flow designer

This is a visual editor for fabrication process steps.

Instead of editing mask geometry, this app edits the process graph:

```text
Clean
Deposit oxide
Spin photoresist
Expose mask
Develop
Etch
Strip resist
Measure
Repeat
```

The app would let a process engineer build and version a route.

Features:

```text
Node-based process editor
Allowed tool classes
Required recipes
Expected inputs and outputs
Hold points
Measurement checkpoints
Branching and rework
Process simulation
```

This can reuse your graph and collaborative editing infrastructure from the mask-layout editor.

MVP:

```text
Drag nodes onto canvas
Connect steps
Attach recipes
Validate that every step has an eligible tool
Export route to MES-lite
```

Advanced:

```text
Cycle-time estimation
Bottleneck analysis
Lot scheduling simulation
Design-of-experiments support
```

This is probably the second most Atomic-aligned app after the mask editor, because it is an interactive editor for a real physical workflow.

## 12. Mask-to-process alignment app

This app connects the mask layout to the process flow.

It answers:

```text
Which mask layer is used at which process step?
Which recipe exposes this layer?
Which design-rule checks apply before this step?
Which wafers used this mask revision?
Which metrology data validates this layer?
```

Your app:

```text
Map mask layers to fab steps
Track mask revisions
Show process dependencies
Warn when process route and mask layout disagree
```

Example:

```text
Mask layer: POLY
Used at step: LITHO_POLY
Required before: POLY_ETCH
DRC deck: poly_rules_v3
Mask revision: RETICLE_A_12
Lots exposed: L-00041, L-00042
```

This is a very good portfolio idea because it links design software to manufacturing software.

## 13. Design-of-experiments planner

Process engineers often vary parameters to learn how a process behaves.

Your app:

```text
Define factors
Define levels
Generate wafer split plan
Assign recipes to wafers
Track results
Analyze response
```

Example:

```text
Experiment: photoresist thickness optimization

Factors:
  spin speed: 3000, 4000, 5000 rpm
  bake temperature: 90, 100, 110 C

Response:
  resist thickness
  line width
  defect count
```

MVP:

```text
Create experiment matrix
Assign wafers
Generate process travelers
Collect measurements
Show plots
```

Advanced:

```text
Full factorial and fractional factorial designs
Randomization
Blocking by tool or day
Response surface modeling
```

This would be impressive because it serves process engineers directly.

## 14. Inventory and materials tracker

Fabs need to track consumables.

Examples:

```text
Wafers
Photoresist
Developers
Solvents
Gases
Targets
Masks
Spare parts
Gloves
Filters
Quartzware
```

Your app:

```text
Material lots
Expiration dates
Storage location
Contamination class
Supplier
Certificate of analysis
Usage history
Low-stock alerts
Material-to-wafer genealogy
```

Example:

```text
Photoresist PR-001
  bottle lot: B-7782
  opened: 2026-05-01
  expires: 2026-06-01
  used on lots: L-0041, L-0042
```

This is less flashy, but useful because traceability matters.

## 15. Maintenance and calibration manager

Equipment needs maintenance and calibration.

Your app:

```text
Maintenance schedules
Calibration records
Tool qualification
Preventive maintenance checklists
Downtime tracking
Spare parts usage
Post-maintenance qualification runs
```

Example:

```text
Tool: Etcher-01
  PM due: May 20, 2026
  Last chamber clean: May 3, 2026
  Last qualification wafer: passed
  Current status: released to production
```

Pair this with the tool dashboard and MES-lite.

Advanced:

```text
Maintenance automatically triggered by run count
Tool locked out after failed calibration
Qualification wafer results linked to release state
```

## 16. Cleanroom environmental monitor

This app tracks facility conditions.

Examples:

```text
Temperature
Humidity
Particle counts
Vibration
Air pressure
Chemical cabinet status
Gas cabinet status
DI water resistivity
Exhaust status
```

Your app:

```text
Live sensor dashboard
Historical trends
Alarm thresholds
Event annotations
Correlation with process excursions
```

MVP:

```text
Synthetic cleanroom sensors
Live dashboard
Threshold alarms
Historical graph
```

Advanced:

```text
Correlate humidity with lithography defects
Correlate vibration with alignment failures
Facility-event timeline
```

This adds another real-world dimension.

## 17. Lab notebook with structured data links

A normal notebook stores prose. A fab notebook should link to real process objects.

Your app:

```text
Experiment notes
Linked wafers
Linked lots
Linked recipes
Linked images
Linked metrology
Linked tool runs
Markdown editor
Timeline view
Search
```

Example entry:

```text
Experiment E-0042
  Goal: reduce poly etch residue
  Linked lots: L-00100, L-00101
  Changed recipe: POLY_ETCH_003 → POLY_ETCH_004
  Result: residue reduced, line width shifted by 15 nm
```

This would be relatively easy to build and very useful for demos.

## 18. Yield analysis app

Yield software connects manufacturing history to final test results.

Your app:

```text
Import electrical test results
Map failures to wafer positions
Correlate failures with process steps
Compare lots
Compare recipes
Find spatial patterns
```

Example:

```text
Lot L-00042
  Yield: 78 percent
  Main failure: ring oscillator frequency too low
  Spatial pattern: edge-heavy
  Suspect: oxide thickness nonuniformity
```

MVP:

```text
Generate synthetic test data
Render wafer yield map
Filter by test
Compare against process measurements
```

Advanced:

```text
Failure clustering
Process correlation matrix
Wafer genealogy drilldown
Interactive root-cause tree
```

This is a strong data visualization app.

## 19. 3D process cross-section simulator

This would be a visually impressive app.

Input:

```text
Process sequence
Layer stack
Etch/deposit steps
Mask openings
```

Output:

```text
Approximate 2D or 3D cross-section
```

Example:

```text
Start with silicon
Grow oxide
Deposit poly
Pattern poly
Etch exposed poly
Deposit metal
Etch metal
Show final cross-section
```

This maps very closely to Atomic Semi’s mention of rendering complex 3D fields and simulation-style visualization. 

MVP:

```text
2D cross-section renderer
Simple deposition and etch operations
Step slider
Layer coloring
```

Advanced:

```text
Voxel grid
GPU volume rendering
Signed distance fields
Interactive process editing
Compare intended vs simulated geometry
```

This is one of the highest-impact visual projects.

## 20. GDS/OASIS inspection and diff tool

This is a companion to the mask-layout editor.

Your app:

```text
Open two layout revisions
Show geometry differences
Highlight changed polygons
Compare bounding boxes
Compare layer counts
Compare cell hierarchy
Generate review report
```

OpenROAD is an open-source digital design project that includes an RTL-to-GDSII flow and physical-design tooling, which makes GDS-style layout output part of a broader chip-design flow. ([GitHub][6])

MVP:

```text
Import your own simplified layout format
Compare revisions
Show added, removed, and modified shapes
```

Advanced:

```text
Real GDSII import
Hierarchy-aware diff
Viewport-linked review comments
DRC delta report
```

This is very portfolio-friendly because diffs, large geometry, and review workflows are software-heavy.

## 21. Wafer genealogy browser

This app answers provenance questions.

```text
Which wafers used this recipe?
Which wafers touched this tool?
Which lots used this material bottle?
Which test failures share a common process step?
Which mask revision produced this wafer?
```

View it as a graph.

Nodes:

```text
Lot
Wafer
Tool run
Recipe version
Material batch
Mask revision
Measurement
Defect
Test result
Operator
```

Edges:

```text
processed_by
used_recipe
used_material
measured_by
derived_from
failed_test
```

This could be a beautiful graph app with spatial filtering and search.

MVP:

```text
Graph database-like model in Postgres
Interactive graph viewer
Search by wafer, lot, recipe, or tool
```

Advanced:

```text
Root-cause path search
Common-ancestor analysis
Impact analysis
```

Example:

```text
Show all wafers affected by photoresist bottle PR-B7782.
```

This is exactly the sort of internal tool that can save time in a fab.

## 22. Scheduler and dispatch app

A fab scheduler decides what should run next.

Inputs:

```text
Lots waiting
Tool availability
Recipe eligibility
Due dates
Maintenance windows
Setup times
Operator availability
Priority lots
```

Outputs:

```text
Recommended next lot for each tool
Estimated completion time
Bottleneck view
Queue times
```

MVP:

```text
Rule-based dispatcher
FIFO plus priority
Tool compatibility
Gantt chart
```

Advanced:

```text
Constraint solver
What-if simulation
Batch tools
Queue-time limits
Setup minimization
```

This is algorithmically interesting and shows graph/search/optimization ability.

## 23. Safety and interlock dashboard

A fab has safety-critical systems, although a portfolio version should stay simulated.

Your app:

```text
Chemical cabinet status
Gas line status
Emergency stop state
Exhaust state
Door interlocks
Tool lockout state
Incident log
```

Keep it as monitoring and simulated interlocks, not actual hazardous equipment control.

MVP:

```text
Simulated safety sensors
Alarm routing
Tool lockout when safety condition fails
Audit trail
```

This shows maturity, but do not make it your main project.

## How these apps fit together

A good architecture would look like this:

```text
                      ┌────────────────────┐
                      │ Mask Layout Editor │
                      └─────────┬──────────┘
                                │
┌──────────────┐      ┌─────────▼──────────┐      ┌──────────────────┐
│ Recipe Mgr   │─────▶│ Process Traveler   │◀────▶│ Scheduler        │
└──────────────┘      └─────────┬──────────┘      └──────────────────┘
                                │
                      ┌─────────▼──────────┐
                      │ Equipment Gateway  │
                      └─────────┬──────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
┌───────▼──────┐       ┌────────▼────────┐      ┌───────▼────────┐
│ Tool Monitor │       │ Run Data Store  │      │ FDC Dashboard  │
└──────────────┘       └────────┬────────┘      └────────────────┘
                                │
                      ┌─────────▼──────────┐
                      │ Metrology Viewer   │
                      └─────────┬──────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
┌───────▼──────┐       ┌────────▼────────┐      ┌───────▼────────┐
│ SPC          │       │ Yield Analysis  │      │ Genealogy      │
└──────────────┘       └─────────────────┘      └────────────────┘
```

This is the story:

```text
The user designs a mask.
The process engineer creates a route.
The recipe manager controls tool parameters.
The MES sends work to equipment.
The equipment simulator produces run data.
The metrology viewer attaches measurements.
The SPC/FDC apps detect drift or faults.
The yield app shows whether the process worked.
The genealogy browser explains what happened.
```

## What I would actually build first

Do not start with all 23. Build a tight suite of 5 apps that share infrastructure.

### Phase 1: visual and interactive

```text
1. Collaborative mask-layout editor
2. Process-flow designer
3. GDS/layout diff viewer
```

This shows CAD-like interaction, computational geometry, graph editing, and real-time collaboration.

### Phase 2: fab operations

```text
4. MES-lite process traveler
5. Recipe manager
6. Equipment simulator
7. Tool dashboard
```

This turns the project into a mini fab operating system.

### Phase 3: data and process control

```text
8. Metrology viewer
9. Wafer map viewer
10. SPC dashboard
11. FDC dashboard
12. Yield analysis
```

This shows visualization, time-series analysis, and manufacturing intelligence.

## Highest-value portfolio apps

If the goal is to impress Atomic Semi specifically, I would rank them like this:

| Rank | App                                     | Why it matters                                                                           |
| ---: | --------------------------------------- | ---------------------------------------------------------------------------------------- |
|    1 | Collaborative mask-layout editor        | Directly matches the CAD/Figma-like geometry editor requirement.                         |
|    2 | Process-flow designer                   | Another interactive editor, but for fab process graphs.                                  |
|    3 | Equipment simulator plus tool dashboard | Shows real-time systems, equipment state, event streams, and physical process awareness. |
|    4 | Metrology and wafer-map viewer          | Strong visualization and measurement story.                                              |
|    5 | SPC/FDC dashboard                       | Shows process engineering relevance and data analysis.                                   |
|    6 | Recipe manager                          | Demonstrates versioning, validation, schema evolution, and auditability.                 |
|    7 | MES-lite traveler                       | Shows you understand fab operations end to end.                                          |
|    8 | 3D process cross-section simulator      | Visually impressive and relevant to simulation/visualization.                            |

## The strongest single portfolio concept

Build:

```text
ReticleStitch FabOS
A Rust/WASM software suite for a simulated semiconductor fab.
```

Include these modules:

```text
Collaborative mask-layout editor
Process-flow designer
Recipe manager
MES-lite wafer traveler
Equipment simulator
Tool dashboard
Metrology and wafer-map viewer
SPC/FDC dashboard
Yield analysis
```

Then make one integrated demo:

```text
1. Draw a simple mask layer.
2. Create a process route that uses that mask.
3. Create and approve a recipe.
4. Start a wafer lot.
5. Dispatch it to a simulated tool.
6. Stream live sensor data.
7. Inject a fault.
8. Record metrology.
9. Show the wafer map.
10. Detect drift in SPC.
11. Link the yield issue back to the tool run and recipe version.
```

That would look much stronger than many separate apps. It would show that you can build the software nervous system of a small fab.

[1]: https://www.isa.org/standards-and-publications/isa-standards/isa-95-standard?utm_source=chatgpt.com "ISA-95 Standard: Enterprise-Control System Integration"
[2]: https://www.semi.org/en/standards-watch-2022-Sept/intro-to-semi-communication-standards?utm_source=chatgpt.com "Introduction to SEMI's Communication Standards: SECS ..."
[3]: https://www.kla.com/products/software-solutions/semiconductor?utm_source=chatgpt.com "Semiconductor Software Solutions"
[4]: https://www.semi.org/en/most-important-qm-tool-statistical-process-control-spc?utm_source=chatgpt.com "The Most Important QM Tool: Statistical Process Control ..."
[5]: https://www.aiche.org/sites/default/files/community/446171/aiche-community-site-page/448906/sondermanspanosdiscussionaichefinal102505.pdf?utm_source=chatgpt.com "Advanced Process Control in Semiconductor Manufacturing"
[6]: https://github.com/The-OpenROAD-Project/OpenROAD?utm_source=chatgpt.com "OpenROAD"

#![allow(unused_imports)]
use super::*;

pub(crate) type ConnectivityLayerSet = BTreeSet<LayerId>;
pub(crate) type ConnectivityLayerPairs = BTreeSet<(LayerId, LayerId)>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConnectivityReport {
    pub components: Vec<NetComponent>,
    pub shape_to_component: BTreeMap<ShapeOccurrenceId, usize>,
    pub devices: Vec<ExtractedDevice>,
    pub shorts: Vec<NetShort>,
    pub opens: Vec<NetOpen>,
    pub skipped: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpiceNetlistExportReport {
    pub component_count: usize,
    pub device_count: usize,
    pub named_net_count: usize,
    pub generated_net_count: usize,
    pub short_count: usize,
    pub open_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpiceNetlistExportResult {
    pub text: String,
    pub report: SpiceNetlistExportReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpiceSchematicNetlist {
    pub circuit_name: String,
    pub pins: Vec<String>,
    pub referenced_nets: Vec<String>,
    pub devices: Vec<SpiceSchematicDevice>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpiceSchematicDevice {
    pub name: String,
    pub kind: String,
    pub nets: Vec<String>,
    pub model: Option<String>,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpiceNetlistParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for SpiceNetlistParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line == 0 {
            write!(formatter, "{}", self.message)
        } else {
            write!(formatter, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for SpiceNetlistParseError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpiceConnectivityComparisonStatus {
    Match,
    LayoutIssues,
    Mismatch,
}

impl SpiceConnectivityComparisonStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Match => "match",
            Self::LayoutIssues => "layout issues",
            Self::Mismatch => "mismatch",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpiceConnectivityComparisonReport {
    pub circuit_name: String,
    pub layout_component_count: usize,
    pub layout_device_count: usize,
    pub layout_named_nets: Vec<String>,
    pub schematic_pins: Vec<String>,
    pub schematic_referenced_nets: Vec<String>,
    pub schematic_device_count: usize,
    pub layout_device_signatures: Vec<String>,
    pub schematic_device_signatures: Vec<String>,
    pub missing_layout_devices: Vec<String>,
    pub extra_layout_devices: Vec<String>,
    pub missing_layout_nets: Vec<String>,
    pub extra_layout_nets: Vec<String>,
    pub layout_short_count: usize,
    pub layout_open_count: usize,
    pub status: SpiceConnectivityComparisonStatus,
}

impl SpiceConnectivityComparisonReport {
    pub fn mismatch_count(&self) -> usize {
        self.missing_layout_nets.len()
            + self.extra_layout_nets.len()
            + self.missing_layout_devices.len()
            + self.extra_layout_devices.len()
    }

    pub fn layout_issue_count(&self) -> usize {
        self.layout_short_count + self.layout_open_count
    }

    pub fn problem_count(&self) -> usize {
        self.mismatch_count() + self.layout_issue_count()
    }

    pub fn is_match(&self) -> bool {
        self.status == SpiceConnectivityComparisonStatus::Match
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectivityHealth {
    Skipped,
    Clean,
    Shorts,
    Opens,
    ShortsAndOpens,
}

impl ConnectivityHealth {
    pub fn label(self) -> &'static str {
        match self {
            Self::Skipped => "skipped",
            Self::Clean => "clean",
            Self::Shorts => "shorts",
            Self::Opens => "opens",
            Self::ShortsAndOpens => "shorts and opens",
        }
    }
}

impl Default for ConnectivityHealth {
    fn default() -> Self {
        Self::Clean
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConnectivitySummary {
    pub health: ConnectivityHealth,
    pub component_count: usize,
    pub device_count: usize,
    pub labeled_component_count: usize,
    pub short_count: usize,
    pub open_count: usize,
    pub largest_component_shape_count: usize,
    pub displayed_short_count: usize,
    pub displayed_open_count: usize,
    pub omitted_short_count: usize,
    pub omitted_open_count: usize,
    pub skipped: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectivityIssueKind {
    Short,
    Open,
}

impl ConnectivityIssueKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Short => "short",
            Self::Open => "open",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConnectivityIssue {
    Short(NetShort),
    Open(NetOpen),
}

impl ConnectivityIssue {
    pub fn kind(&self) -> ConnectivityIssueKind {
        match self {
            Self::Short(_) => ConnectivityIssueKind::Short,
            Self::Open(_) => ConnectivityIssueKind::Open,
        }
    }

    pub fn bounds(&self) -> Rect {
        match self {
            Self::Short(short) => short.bounds,
            Self::Open(open) => open.bounds,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectivityIssueRecord {
    pub key: String,
    pub issue: ConnectivityIssue,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConnectivityIssueSummary {
    pub total_count: usize,
    pub displayed_count: usize,
    pub omitted_count: usize,
    pub short_count: usize,
    pub open_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConnectivityIssueStore {
    pub records: Vec<ConnectivityIssueRecord>,
    pub(crate) key_index: BTreeMap<String, usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectivityValidationSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectivityValidationFinding {
    pub severity: ConnectivityValidationSeverity,
    pub message: String,
}

impl ConnectivityValidationFinding {
    pub(crate) fn error(message: impl Into<String>) -> Self {
        Self {
            severity: ConnectivityValidationSeverity::Error,
            message: message.into(),
        }
    }

    pub(crate) fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: ConnectivityValidationSeverity::Warning,
            message: message.into(),
        }
    }
}

impl ConnectivityReport {
    pub fn skipped(reason: impl Into<String>) -> Self {
        Self {
            skipped: Some(reason.into()),
            ..Default::default()
        }
    }

    pub fn component(&self, id: usize) -> Option<&NetComponent> {
        id.checked_sub(1)
            .and_then(|index| self.components.get(index))
    }

    pub fn component_for_occurrence(&self, occurrence: &ShapeOccurrenceId) -> Option<usize> {
        self.shape_to_component.get(occurrence).copied()
    }

    pub fn summary(&self, max_issue_rows: usize) -> ConnectivitySummary {
        let displayed_short_count = self.shorts.len().min(max_issue_rows);
        let displayed_open_count = self.opens.len().min(max_issue_rows);
        ConnectivitySummary {
            health: if self.skipped.is_some() {
                ConnectivityHealth::Skipped
            } else if self.shorts.is_empty() && self.opens.is_empty() {
                ConnectivityHealth::Clean
            } else if self.shorts.is_empty() {
                ConnectivityHealth::Opens
            } else if self.opens.is_empty() {
                ConnectivityHealth::Shorts
            } else {
                ConnectivityHealth::ShortsAndOpens
            },
            component_count: self.components.len(),
            device_count: self.devices.len(),
            labeled_component_count: self
                .components
                .iter()
                .filter(|component| component.net_name.is_some())
                .count(),
            short_count: self.shorts.len(),
            open_count: self.opens.len(),
            largest_component_shape_count: self
                .components
                .iter()
                .map(|component| component.shapes.len())
                .max()
                .unwrap_or(0),
            displayed_short_count,
            displayed_open_count,
            omitted_short_count: self.shorts.len().saturating_sub(displayed_short_count),
            omitted_open_count: self.opens.len().saturating_sub(displayed_open_count),
            skipped: self.skipped.clone(),
        }
    }

    pub fn issue_store(&self) -> ConnectivityIssueStore {
        ConnectivityIssueStore::from_report(self)
    }

    pub fn validate(&self) -> Vec<ConnectivityValidationFinding> {
        let mut findings = Vec::new();
        if self
            .skipped
            .as_ref()
            .is_some_and(|reason| reason.trim().is_empty())
        {
            findings.push(ConnectivityValidationFinding::error(
                "connectivity report has an empty skipped reason",
            ));
        }
        if self.skipped.is_some()
            && (!self.components.is_empty()
                || !self.shape_to_component.is_empty()
                || !self.devices.is_empty()
                || !self.shorts.is_empty()
                || !self.opens.is_empty())
        {
            findings.push(ConnectivityValidationFinding::warning(
                "connectivity report is marked skipped but also contains extracted data",
            ));
        }

        let mut component_ids = BTreeSet::new();
        let mut component_shapes = BTreeMap::new();
        let mut component_shape_pairs = BTreeSet::new();
        for (index, component) in self.components.iter().enumerate() {
            let expected_id = index + 1;
            if component.id != expected_id {
                findings.push(ConnectivityValidationFinding::error(format!(
                    "connectivity component at index {index} has id {}, expected {expected_id}",
                    component.id
                )));
            }
            if !component_ids.insert(component.id) {
                findings.push(ConnectivityValidationFinding::error(format!(
                    "connectivity component id {} is duplicated",
                    component.id
                )));
            }
            if component.shapes.is_empty() {
                findings.push(ConnectivityValidationFinding::warning(format!(
                    "connectivity component {} has no shapes",
                    component.id
                )));
            }
            let mut seen_shapes = BTreeSet::new();
            for occurrence in &component.shapes {
                if !seen_shapes.insert(occurrence.clone()) {
                    findings.push(ConnectivityValidationFinding::error(format!(
                        "connectivity component {} repeats shape occurrence {:?}",
                        component.id, occurrence
                    )));
                }
                if let Some(mapped_component) = self.shape_to_component.get(occurrence) {
                    if *mapped_component != component.id {
                        findings.push(ConnectivityValidationFinding::error(format!(
                            "connectivity component {} shape {:?} maps to component {}",
                            component.id, occurrence, mapped_component
                        )));
                    }
                } else {
                    findings.push(ConnectivityValidationFinding::error(format!(
                        "connectivity component {} shape {:?} is missing from shape_to_component",
                        component.id, occurrence
                    )));
                }
                if let Some(previous_component) =
                    component_shapes.insert(occurrence.clone(), component.id)
                {
                    findings.push(ConnectivityValidationFinding::error(format!(
                        "connectivity shape {:?} appears in components {} and {}",
                        occurrence, previous_component, component.id
                    )));
                }
                component_shape_pairs.insert((occurrence.clone(), component.id));
            }
            validate_component_names(component, &mut findings);
        }

        for (occurrence, component_id) in &self.shape_to_component {
            if !component_ids.contains(component_id) {
                findings.push(ConnectivityValidationFinding::error(format!(
                    "connectivity shape {occurrence:?} maps to missing component {component_id}"
                )));
            }
            if !component_shape_pairs.contains(&(occurrence.clone(), *component_id)) {
                findings.push(ConnectivityValidationFinding::error(format!(
                    "connectivity shape {occurrence:?} maps to component {component_id}, but no matching component shape entry exists"
                )));
            }
        }

        let mut device_ids = BTreeSet::new();
        for (index, device) in self.devices.iter().enumerate() {
            let expected_id = index + 1;
            if device.id != expected_id {
                findings.push(ConnectivityValidationFinding::error(format!(
                    "extracted device at index {index} has id {}, expected {expected_id}",
                    device.id
                )));
            }
            if !device_ids.insert(device.id) {
                findings.push(ConnectivityValidationFinding::error(format!(
                    "extracted device id {} is duplicated",
                    device.id
                )));
            }
            if device.kind.trim().is_empty() {
                findings.push(ConnectivityValidationFinding::error(format!(
                    "extracted device {} has an empty kind",
                    device.id
                )));
            }
            if device.model.trim().is_empty() {
                findings.push(ConnectivityValidationFinding::warning(format!(
                    "extracted device {} has an empty model",
                    device.id
                )));
            }
            if device.terminals.is_empty() {
                findings.push(ConnectivityValidationFinding::error(format!(
                    "extracted device {} has no terminals",
                    device.id
                )));
            }
            let mut terminal_names = BTreeSet::new();
            for terminal in &device.terminals {
                if terminal.name.trim().is_empty() {
                    findings.push(ConnectivityValidationFinding::error(format!(
                        "extracted device {} has an empty terminal name",
                        device.id
                    )));
                } else if !terminal_names.insert(terminal.name.clone()) {
                    findings.push(ConnectivityValidationFinding::error(format!(
                        "extracted device {} repeats terminal {:?}",
                        device.id, terminal.name
                    )));
                }
                if let Some(component) = terminal.component
                    && !component_ids.contains(&component)
                {
                    findings.push(ConnectivityValidationFinding::error(format!(
                        "extracted device {} terminal {:?} references missing component {}",
                        device.id, terminal.name, component
                    )));
                }
                if terminal
                    .net_name
                    .as_ref()
                    .is_some_and(|name| name.trim().is_empty())
                {
                    findings.push(ConnectivityValidationFinding::error(format!(
                        "extracted device {} terminal {:?} has an empty net name",
                        device.id, terminal.name
                    )));
                }
            }
            if device.bounds.width() < 0 || device.bounds.height() < 0 {
                findings.push(ConnectivityValidationFinding::error(format!(
                    "extracted device {} has inverted bounds {:?}",
                    device.id, device.bounds
                )));
            }
            if device.width <= 0 || device.length <= 0 {
                findings.push(ConnectivityValidationFinding::warning(format!(
                    "extracted device {} has non-positive dimensions W={} L={}",
                    device.id, device.width, device.length
                )));
            }
        }

        let mut issue_keys = BTreeSet::new();
        for short in &self.shorts {
            validate_short(short, &component_ids, &mut issue_keys, &mut findings);
        }
        for open in &self.opens {
            validate_open(open, &component_ids, &mut issue_keys, &mut findings);
        }

        findings
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetComponent {
    pub id: usize,
    pub shapes: Vec<ShapeOccurrenceId>,
    pub bounds: Rect,
    pub labels: Vec<NetLabel>,
    pub explicit_nets: Vec<NetId>,
    pub net_name: Option<String>,
    pub net_id: Option<NetId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractedDevice {
    pub id: usize,
    pub kind: String,
    pub model: String,
    pub terminals: Vec<ExtractedDeviceTerminal>,
    pub bounds: Rect,
    pub width: Coord,
    pub length: Coord,
    pub occurrences: Vec<ShapeOccurrenceId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractedDeviceTerminal {
    pub name: String,
    pub component: Option<usize>,
    pub net_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetLabel {
    pub occurrence: ShapeOccurrenceId,
    pub text: String,
    pub position: Point,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetShort {
    pub component: usize,
    pub names: Vec<String>,
    pub bounds: Rect,
}

impl NetShort {
    pub fn stable_key(&self) -> String {
        let mut names = self.names.clone();
        names.sort();
        format!(
            "short|{}|{},{},{},{}",
            names.join(","),
            self.bounds.min.x,
            self.bounds.min.y,
            self.bounds.max.x,
            self.bounds.max.y
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetOpen {
    pub name: String,
    pub components: Vec<usize>,
    pub bounds: Rect,
}

impl NetOpen {
    pub fn stable_key(&self) -> String {
        format!(
            "open|{}|{},{},{},{}",
            self.name, self.bounds.min.x, self.bounds.min.y, self.bounds.max.x, self.bounds.max.y
        )
    }
}

impl ConnectivityIssueRecord {
    pub fn kind(&self) -> ConnectivityIssueKind {
        self.issue.kind()
    }

    pub fn bounds(&self) -> Rect {
        self.issue.bounds()
    }
}

impl ConnectivityIssueStore {
    pub fn from_report(report: &ConnectivityReport) -> Self {
        let mut records = Vec::with_capacity(report.shorts.len() + report.opens.len());
        records.extend(
            report
                .shorts
                .iter()
                .cloned()
                .map(|short| ConnectivityIssueRecord {
                    key: short.stable_key(),
                    issue: ConnectivityIssue::Short(short),
                }),
        );
        records.extend(
            report
                .opens
                .iter()
                .cloned()
                .map(|open| ConnectivityIssueRecord {
                    key: open.stable_key(),
                    issue: ConnectivityIssue::Open(open),
                }),
        );

        let mut key_index = BTreeMap::new();
        for (index, record) in records.iter().enumerate() {
            key_index.entry(record.key.clone()).or_insert(index);
        }
        Self { records, key_index }
    }

    pub fn get(&self, key: &str) -> Option<&ConnectivityIssueRecord> {
        self.key_index
            .get(key)
            .and_then(|index| self.records.get(*index))
    }

    pub fn displayed_records(&self, max_issue_rows: usize) -> &[ConnectivityIssueRecord] {
        let displayed_count = self.records.len().min(max_issue_rows);
        &self.records[..displayed_count]
    }

    pub fn summary(&self, max_issue_rows: usize) -> ConnectivityIssueSummary {
        let displayed_count = self.records.len().min(max_issue_rows);
        let short_count = self
            .records
            .iter()
            .filter(|record| record.kind() == ConnectivityIssueKind::Short)
            .count();
        let open_count = self.records.len().saturating_sub(short_count);
        ConnectivityIssueSummary {
            total_count: self.records.len(),
            displayed_count,
            omitted_count: self.records.len().saturating_sub(displayed_count),
            short_count,
            open_count,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ConnectiveShape {
    pub(crate) index: usize,
    pub(crate) occurrence: ShapeOccurrenceId,
    pub(crate) shape: Shape,
    pub(crate) bounds: Rect,
    pub(crate) layers: BTreeSet<LayerId>,
}

#[derive(Clone, Debug)]
pub(crate) struct LabelShape {
    pub(crate) occurrence: ShapeOccurrenceId,
    pub(crate) layer: LayerId,
    pub(crate) text: String,
    pub(crate) position: Point,
}

#[derive(Clone, Debug)]
pub(crate) struct IndexedConnectiveShape {
    pub(crate) index: usize,
    pub(crate) bounds: Rect,
}

impl RTreeObject for IndexedConnectiveShape {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(
            [self.bounds.min.x as f64, self.bounds.min.y as f64],
            [self.bounds.max.x as f64, self.bounds.max.y as f64],
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) struct UnionFind {
    pub(crate) parent: Vec<usize>,
    pub(crate) rank: Vec<u8>,
}

impl UnionFind {
    pub(crate) fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
            rank: vec![0; len],
        }
    }

    pub(crate) fn find(&mut self, value: usize) -> usize {
        if self.parent[value] != value {
            let root = self.find(self.parent[value]);
            self.parent[value] = root;
        }
        self.parent[value]
    }

    pub(crate) fn union(&mut self, left: usize, right: usize) {
        let left = self.find(left);
        let right = self.find(right);
        if left == right {
            return;
        }
        if self.rank[left] < self.rank[right] {
            self.parent[left] = right;
        } else if self.rank[left] > self.rank[right] {
            self.parent[right] = left;
        } else {
            self.parent[right] = left;
            self.rank[left] += 1;
        }
    }
}

pub fn extract_connectivity(
    document: &Document,
    technology: &TechnologyFile,
) -> Result<ConnectivityReport, TechnologyError> {
    technology.validate()?;
    let (conductive_layers, connected_pairs) = connectivity_layers(document, technology)?;
    let (connective_shapes, labels) = materialize_shapes(document, &conductive_layers);
    if connective_shapes.is_empty() {
        let devices = extract_devices(document, &[], &BTreeMap::new());
        if devices.is_empty() {
            return Ok(ConnectivityReport::default());
        }
        return Ok(ConnectivityReport {
            devices,
            ..Default::default()
        });
    }

    let entries: Vec<_> = connective_shapes
        .iter()
        .map(|shape| IndexedConnectiveShape {
            index: shape.index,
            bounds: shape.bounds,
        })
        .collect();
    let tree = RTree::bulk_load(entries);
    let mut union_find = UnionFind::new(connective_shapes.len());

    for shape in &connective_shapes {
        let envelope = AABB::from_corners(
            [shape.bounds.min.x as f64, shape.bounds.min.y as f64],
            [shape.bounds.max.x as f64, shape.bounds.max.y as f64],
        );
        for candidate in tree.locate_in_envelope_intersecting(&envelope) {
            if candidate.index <= shape.index {
                continue;
            }
            let other = &connective_shapes[candidate.index];
            if shape.bounds.intersects(other.bounds)
                && layers_are_connected(&shape.layers, &other.layers, &connected_pairs)
            {
                union_find.union(shape.index, other.index);
            }
        }
    }

    let mut roots = BTreeMap::<usize, Vec<usize>>::new();
    for shape in &connective_shapes {
        let root = union_find.find(shape.index);
        roots.entry(root).or_default().push(shape.index);
    }

    let mut root_to_component = BTreeMap::new();
    let mut components = Vec::new();
    let mut shape_to_component = BTreeMap::new();
    for (root, indexes) in roots {
        let id = components.len() + 1;
        root_to_component.insert(root, id);
        let mut shapes = Vec::new();
        let mut bounds = connective_shapes[indexes[0]].bounds;
        let mut explicit_nets = BTreeSet::new();
        for index in indexes {
            let shape = &connective_shapes[index];
            shapes.push(shape.occurrence.clone());
            bounds = bounds.union(shape.bounds);
            if let Some(net) = shape.shape.net {
                explicit_nets.insert(net);
            }
            shape_to_component.insert(shape.occurrence.clone(), id);
        }
        components.push(NetComponent {
            id,
            shapes,
            bounds,
            labels: Vec::new(),
            explicit_nets: explicit_nets.into_iter().collect(),
            net_name: None,
            net_id: None,
        });
    }

    assign_labels(
        &mut components,
        &connective_shapes,
        &labels,
        &mut union_find,
        &root_to_component,
    );
    let shorts = summarize_component_names(&mut components);
    let opens = find_open_nets(&components);
    let devices = extract_devices(document, &components, &shape_to_component);

    Ok(ConnectivityReport {
        components,
        shape_to_component,
        devices,
        shorts,
        opens,
        skipped: None,
    })
}

pub fn export_connectivity_spice(
    circuit_name: &str,
    report: &ConnectivityReport,
) -> SpiceNetlistExportResult {
    let mut export_report = SpiceNetlistExportReport {
        component_count: report.components.len(),
        device_count: report.devices.len(),
        short_count: report.shorts.len(),
        open_count: report.opens.len(),
        ..Default::default()
    };
    let circuit_name = sanitize_spice_identifier(circuit_name, "glassworks");
    let mut component_nets = BTreeMap::new();
    for component in &report.components {
        let (net_name, generated) = spice_component_net_name(component);
        if generated {
            export_report.generated_net_count += 1;
        } else {
            export_report.named_net_count += 1;
        }
        component_nets.insert(component.id, net_name);
    }

    let mut text = String::new();
    text.push_str("* Glassworks connectivity SPICE subset export\n");
    text.push_str(&format!("* Circuit: {circuit_name}\n"));
    text.push_str(&format!(
        "* Components: {} devices={} named_nets={} generated_nets={} shorts={} opens={}\n",
        export_report.component_count,
        export_report.device_count,
        export_report.named_net_count,
        export_report.generated_net_count,
        export_report.short_count,
        export_report.open_count
    ));
    if let Some(skipped) = &report.skipped {
        text.push_str(&format!(
            "* Skipped connectivity extraction: {}\n",
            spice_comment(skipped)
        ));
        export_report
            .warnings
            .push("connectivity report was marked skipped".to_string());
    }

    let pins = spice_subckt_pins(&component_nets, &report.devices);
    write_spice_subckt_line(&mut text, &circuit_name, &pins);
    for component in &report.components {
        let net_name = component_nets
            .get(&component.id)
            .cloned()
            .unwrap_or_else(|| format!("N_{}", component.id));
        let labels = component
            .labels
            .iter()
            .map(|label| spice_comment(&label.text))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(",");
        let labels = if labels.is_empty() {
            "-".to_string()
        } else {
            labels
        };
        text.push_str(&format!(
            "* component {} net={} shapes={} labels={} bounds={}\n",
            component.id,
            net_name,
            component.shapes.len(),
            labels,
            spice_rect(component.bounds)
        ));
    }
    for device in &report.devices {
        if device.kind.eq_ignore_ascii_case("mos") {
            let drain = spice_device_terminal_net(device, "D", &component_nets);
            let gate = spice_device_terminal_net(device, "G", &component_nets);
            let source = spice_device_terminal_net(device, "S", &component_nets);
            let body = spice_device_terminal_net(device, "B", &component_nets);
            text.push_str(&format!(
                "MDEV{} {} {} {} {} {} L={} W={}\n",
                device.id,
                drain,
                gate,
                source,
                body,
                sanitize_spice_identifier(&device.model, "NMOS"),
                device.length,
                device.width
            ));
        } else if device.kind.eq_ignore_ascii_case("resistor") {
            let a = spice_device_terminal_net(device, "A", &component_nets);
            let b = spice_device_terminal_net(device, "B", &component_nets);
            text.push_str(&format!("RDEV{} {} {} 1\n", device.id, a, b));
        } else if device.kind.eq_ignore_ascii_case("capacitor") {
            let a = spice_device_terminal_net(device, "A", &component_nets);
            let b = spice_device_terminal_net(device, "B", &component_nets);
            text.push_str(&format!("CDEV{} {} {} 1\n", device.id, a, b));
        } else {
            export_report.warnings.push(format!(
                "device {} kind {:?} is not supported by SPICE export",
                device.id, device.kind
            ));
            text.push_str(&format!(
                "* skipped device {} kind={} terminals={}\n",
                device.id,
                spice_comment(&device.kind),
                device.terminals.len()
            ));
        }
    }
    for short in &report.shorts {
        text.push_str(&format!(
            "* short component={} nets={} bounds={}\n",
            short.component,
            short
                .names
                .iter()
                .map(|name| sanitize_spice_identifier(name, "NET"))
                .collect::<Vec<_>>()
                .join(","),
            spice_rect(short.bounds)
        ));
    }
    for open in &report.opens {
        text.push_str(&format!(
            "* open net={} components={} bounds={}\n",
            sanitize_spice_identifier(&open.name, "NET"),
            open.components
                .iter()
                .map(|component| component.to_string())
                .collect::<Vec<_>>()
                .join(","),
            spice_rect(open.bounds)
        ));
    }
    text.push_str(&format!(".ends {circuit_name}\n"));
    text.push_str(".end\n");

    SpiceNetlistExportResult {
        text,
        report: export_report,
    }
}

pub fn parse_spice_schematic_netlist(
    text: &str,
) -> Result<SpiceSchematicNetlist, SpiceNetlistParseError> {
    let mut circuit_name = None;
    let mut pins = Vec::new();
    let mut referenced_nets = Vec::new();
    let mut devices = Vec::new();
    let mut in_subckt = false;
    let mut saw_end = false;

    for (line_number, line) in spice_logical_lines(text) {
        let line = strip_spice_inline_comment(&line);
        let line = line.trim();
        if line.is_empty() || line.starts_with('*') {
            continue;
        }
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        if tokens.is_empty() {
            continue;
        }
        let directive = tokens[0].to_ascii_lowercase();
        if directive == ".subckt" {
            if circuit_name.is_some() {
                return Err(SpiceNetlistParseError {
                    line: line_number,
                    message: "multiple .subckt definitions are not supported".to_string(),
                });
            }
            if tokens.len() < 2 {
                return Err(SpiceNetlistParseError {
                    line: line_number,
                    message: ".subckt is missing a circuit name".to_string(),
                });
            }
            let name = sanitize_spice_identifier(tokens[1], "SCHEMATIC");
            let parsed_pins = tokens[2..]
                .iter()
                .map(|pin| sanitize_spice_identifier(pin, "NET"))
                .collect::<Vec<_>>();
            referenced_nets.extend(parsed_pins.iter().cloned());
            circuit_name = Some(name);
            pins = dedupe_spice_identifiers(parsed_pins);
            in_subckt = true;
            continue;
        }
        if directive == ".ends" {
            if in_subckt {
                saw_end = true;
                in_subckt = false;
                break;
            }
            continue;
        }
        if directive == ".end" {
            saw_end = true;
            break;
        }
        if !in_subckt {
            continue;
        }
        if directive.starts_with('.') {
            continue;
        }

        let terminal_count = spice_device_terminal_count(tokens[0], tokens.len() - 1);
        if terminal_count == 0 {
            return Err(SpiceNetlistParseError {
                line: line_number,
                message: format!("device {:?} has no terminals", tokens[0]),
            });
        }
        if tokens.len() <= terminal_count {
            return Err(SpiceNetlistParseError {
                line: line_number,
                message: format!("device {:?} is missing terminal nets", tokens[0]),
            });
        }
        let name = sanitize_spice_identifier(tokens[0], "DEVICE");
        let kind = name.chars().next().unwrap_or('X').to_string();
        let nets = tokens[1..=terminal_count]
            .iter()
            .map(|net| sanitize_spice_identifier(net, "NET"))
            .collect::<Vec<_>>();
        referenced_nets.extend(nets.iter().cloned());
        let model = tokens
            .get(terminal_count + 1)
            .map(|model| sanitize_spice_identifier(model, "MODEL"));
        let parameters = tokens
            .get(terminal_count + 2..)
            .unwrap_or(&[])
            .iter()
            .filter_map(|token| parse_spice_device_parameter(token))
            .collect::<BTreeMap<_, _>>();
        devices.push(SpiceSchematicDevice {
            name,
            kind,
            nets,
            model,
            parameters,
        });
    }

    let Some(circuit_name) = circuit_name else {
        return Err(SpiceNetlistParseError {
            line: 0,
            message: "SPICE netlist does not contain a .subckt definition".to_string(),
        });
    };
    if in_subckt && !saw_end {
        return Err(SpiceNetlistParseError {
            line: 0,
            message: format!("SPICE subckt {circuit_name} is missing .ends"),
        });
    }

    Ok(SpiceSchematicNetlist {
        circuit_name,
        pins,
        referenced_nets: dedupe_spice_identifiers(referenced_nets),
        devices,
    })
}

pub fn compare_connectivity_to_spice_schematic(
    report: &ConnectivityReport,
    schematic: &SpiceSchematicNetlist,
) -> SpiceConnectivityComparisonReport {
    let layout_named_nets = layout_spice_named_nets(report);
    let layout_set = layout_named_nets.iter().cloned().collect::<BTreeSet<_>>();
    let schematic_set = schematic
        .referenced_nets
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let missing_layout_nets = schematic_set
        .difference(&layout_set)
        .cloned()
        .collect::<Vec<_>>();
    let extra_layout_nets = layout_set
        .difference(&schematic_set)
        .cloned()
        .collect::<Vec<_>>();
    let schematic_device_signatures = schematic_spice_device_signatures(schematic);
    let mos_dimension_keys = schematic_mos_dimension_keys(schematic);
    let layout_device_signatures = layout_spice_device_signatures(report, &mos_dimension_keys);
    let missing_layout_devices =
        signature_multiset_difference(&schematic_device_signatures, &layout_device_signatures);
    let extra_layout_devices =
        signature_multiset_difference(&layout_device_signatures, &schematic_device_signatures);
    let layout_short_count = report.shorts.len();
    let layout_open_count = report.opens.len();
    let status = if missing_layout_nets.is_empty()
        && extra_layout_nets.is_empty()
        && missing_layout_devices.is_empty()
        && extra_layout_devices.is_empty()
    {
        if layout_short_count == 0 && layout_open_count == 0 {
            SpiceConnectivityComparisonStatus::Match
        } else {
            SpiceConnectivityComparisonStatus::LayoutIssues
        }
    } else {
        SpiceConnectivityComparisonStatus::Mismatch
    };

    SpiceConnectivityComparisonReport {
        circuit_name: schematic.circuit_name.clone(),
        layout_component_count: report.components.len(),
        layout_device_count: report.devices.len(),
        layout_named_nets,
        schematic_pins: schematic.pins.clone(),
        schematic_referenced_nets: schematic.referenced_nets.clone(),
        schematic_device_count: schematic.devices.len(),
        layout_device_signatures,
        schematic_device_signatures,
        missing_layout_devices,
        extra_layout_devices,
        missing_layout_nets,
        extra_layout_nets,
        layout_short_count,
        layout_open_count,
        status,
    }
}

pub(crate) fn layout_spice_named_nets(report: &ConnectivityReport) -> Vec<String> {
    let mut names = report
        .components
        .iter()
        .filter_map(|component| component.net_name.as_deref())
        .map(|name| sanitize_spice_identifier(name, "NET"))
        .collect::<Vec<_>>();
    names.extend(
        report
            .devices
            .iter()
            .flat_map(|device| device.terminals.iter())
            .filter_map(|terminal| terminal.net_name.as_deref())
            .map(|name| sanitize_spice_identifier(name, "NET")),
    );
    dedupe_spice_identifiers(names)
}

pub(crate) fn layout_spice_device_signatures(
    report: &ConnectivityReport,
    mos_dimension_keys: &[&'static str],
) -> Vec<String> {
    let mut signatures = report
        .devices
        .iter()
        .filter_map(|device| layout_spice_device_signature(report, device, mos_dimension_keys))
        .collect::<Vec<_>>();
    signatures.sort();
    signatures
}

pub fn layout_spice_device_signature(
    report: &ConnectivityReport,
    device: &ExtractedDevice,
    mos_dimension_keys: &[&'static str],
) -> Option<String> {
    let component_nets = report
        .components
        .iter()
        .map(|component| {
            let (net_name, _) = spice_component_net_name(component);
            (component.id, net_name)
        })
        .collect::<BTreeMap<_, _>>();
    if device.kind.eq_ignore_ascii_case("mos") {
        let params = mos_dimension_keys
            .iter()
            .filter_map(|key| match *key {
                "L" => Some(("L".to_string(), device.length.to_string())),
                "W" => Some(("W".to_string(), device.width.to_string())),
                _ => None,
            })
            .collect::<Vec<_>>();
        return Some(spice_device_signature(
            "M",
            &sanitize_spice_identifier(&device.model, "NMOS"),
            &[
                spice_device_terminal_net(device, "D", &component_nets),
                spice_device_terminal_net(device, "G", &component_nets),
                spice_device_terminal_net(device, "S", &component_nets),
                spice_device_terminal_net(device, "B", &component_nets),
            ],
            &params,
        ));
    }
    if device.kind.eq_ignore_ascii_case("resistor") {
        return Some(spice_passive_device_signature(
            "R",
            "RES",
            &[
                spice_device_terminal_net(device, "A", &component_nets),
                spice_device_terminal_net(device, "B", &component_nets),
            ],
        ));
    }
    if device.kind.eq_ignore_ascii_case("capacitor") {
        return Some(spice_passive_device_signature(
            "C",
            "CAP",
            &[
                spice_device_terminal_net(device, "A", &component_nets),
                spice_device_terminal_net(device, "B", &component_nets),
            ],
        ));
    }
    None
}

pub(crate) fn schematic_spice_device_signatures(schematic: &SpiceSchematicNetlist) -> Vec<String> {
    let mut signatures = schematic
        .devices
        .iter()
        .filter_map(|device| {
            if device.kind != "M" || device.nets.len() < 4 {
                if device.kind == "R" && device.nets.len() >= 2 {
                    return Some(spice_passive_device_signature(
                        "R",
                        "RES",
                        &device.nets[..2],
                    ));
                }
                if device.kind == "C" && device.nets.len() >= 2 {
                    return Some(spice_passive_device_signature(
                        "C",
                        "CAP",
                        &device.nets[..2],
                    ));
                }
                return None;
            }
            Some(spice_device_signature(
                "M",
                device.model.as_deref().unwrap_or("NMOS"),
                &device.nets[..4],
                &schematic_mos_dimension_parameters(device).unwrap_or_default(),
            ))
        })
        .collect::<Vec<_>>();
    signatures.sort();
    signatures
}

pub(crate) fn spice_passive_device_signature(kind: &str, model: &str, nets: &[String]) -> String {
    let mut nets = nets
        .iter()
        .map(|net| sanitize_spice_identifier(net, "NET"))
        .collect::<Vec<_>>();
    nets.sort();
    spice_device_signature(kind, model, &nets, &[])
}

pub(crate) fn spice_device_signature(
    kind: &str,
    model: &str,
    nets: &[String],
    parameters: &[(String, String)],
) -> String {
    let nets = nets
        .iter()
        .map(|net| sanitize_spice_identifier(net, "NET"))
        .collect::<Vec<_>>()
        .join(",");
    let mut signature = format!(
        "{}|{}|{}",
        sanitize_spice_identifier(kind, "DEVICE"),
        sanitize_spice_identifier(model, "MODEL"),
        nets
    );
    if !parameters.is_empty() {
        let params = parameters
            .iter()
            .map(|(key, value)| {
                format!(
                    "{}={}",
                    sanitize_spice_identifier(key, "PARAM"),
                    sanitize_spice_parameter_value(value)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        signature.push('|');
        signature.push_str(&params);
    }
    signature
}

pub(crate) fn schematic_mos_dimension_parameters(
    device: &SpiceSchematicDevice,
) -> Option<Vec<(String, String)>> {
    let mut params = Vec::new();
    for key in ["L", "W"] {
        if let Some(value) = device.parameters.get(key) {
            params.push((key.to_string(), value.clone()));
        }
    }
    (!params.is_empty()).then_some(params)
}

pub(crate) fn schematic_mos_dimension_keys(schematic: &SpiceSchematicNetlist) -> Vec<&'static str> {
    ["L", "W"]
        .iter()
        .copied()
        .filter(|key| {
            schematic
                .devices
                .iter()
                .any(|device| device.kind == "M" && device.parameters.contains_key(*key))
        })
        .collect()
}

pub(crate) fn signature_multiset_difference(left: &[String], right: &[String]) -> Vec<String> {
    let mut right_counts = BTreeMap::<&str, usize>::new();
    for signature in right {
        *right_counts.entry(signature.as_str()).or_insert(0) += 1;
    }
    let mut diff = Vec::new();
    for signature in left {
        match right_counts.get_mut(signature.as_str()) {
            Some(count) if *count > 0 => *count -= 1,
            _ => diff.push(signature.clone()),
        }
    }
    diff
}

#[derive(Clone, Debug)]
pub(crate) struct DeviceShape {
    pub(crate) occurrence: ShapeOccurrenceId,
    pub(crate) layer: LayerId,
    pub(crate) bounds: Rect,
}

pub(crate) fn extract_devices(
    document: &Document,
    components: &[NetComponent],
    shape_to_component: &BTreeMap<ShapeOccurrenceId, usize>,
) -> Vec<ExtractedDevice> {
    let Some(diffusion_layer) = document.layer_by_process(ProcessLayer::Diffusion) else {
        return Vec::new();
    };
    let Some(poly_layer) = document.layer_by_process(ProcessLayer::Poly) else {
        return Vec::new();
    };
    let (diffusion_shapes, poly_shapes, body_shapes, labels) =
        materialize_device_shapes(document, diffusion_layer, poly_layer);
    let mut devices = Vec::new();
    for diffusion in &diffusion_shapes {
        for poly in &poly_shapes {
            let Some(channel) = diffusion.bounds.intersection(poly.bounds) else {
                continue;
            };
            if channel.width() <= 0 || channel.height() <= 0 {
                continue;
            }
            let diffusion_component = shape_to_component.get(&diffusion.occurrence).copied();
            let (drain_region, source_region) =
                diffusion_terminal_regions(diffusion.bounds, channel);
            let drain_terminal = extracted_terminal_for_region(
                "D",
                diffusion_component,
                components,
                &labels,
                diffusion.layer,
                drain_region,
            );
            let source_terminal = extracted_terminal_for_region(
                "S",
                diffusion_component,
                components,
                &labels,
                diffusion.layer,
                source_region,
            );
            let gate_terminal = ExtractedDeviceTerminal {
                name: "G".to_string(),
                component: None,
                net_name: label_net_for_device_shape(&labels, poly.layer, poly.bounds),
            };
            let (body_terminal, body_occurrence) =
                mos_body_terminal_for_channel(&body_shapes, &labels, channel);
            let mut occurrences = vec![diffusion.occurrence.clone(), poly.occurrence.clone()];
            if let Some(occurrence) = body_occurrence {
                occurrences.push(occurrence);
            }
            devices.push(ExtractedDevice {
                id: devices.len() + 1,
                kind: "mos".to_string(),
                model: "nmos".to_string(),
                terminals: vec![
                    drain_terminal,
                    gate_terminal,
                    source_terminal,
                    body_terminal,
                ],
                bounds: channel,
                width: channel.width().max(channel.height()),
                length: channel.width().min(channel.height()),
                occurrences,
            });
        }
    }
    for poly in &poly_shapes {
        let overlaps_diffusion = diffusion_shapes.iter().any(|diffusion| {
            poly.bounds
                .intersection(diffusion.bounds)
                .is_some_and(|overlap| overlap.width() > 0 && overlap.height() > 0)
        });
        if overlaps_diffusion {
            continue;
        }
        let (a_region, b_region) = resistor_terminal_regions(poly.bounds);
        let Some(a_net) = label_net_for_device_shape(&labels, poly.layer, a_region) else {
            continue;
        };
        let Some(b_net) = label_net_for_device_shape(&labels, poly.layer, b_region) else {
            continue;
        };
        if a_net == b_net {
            continue;
        }
        devices.push(ExtractedDevice {
            id: devices.len() + 1,
            kind: "resistor".to_string(),
            model: "polyres".to_string(),
            terminals: vec![
                ExtractedDeviceTerminal {
                    name: "A".to_string(),
                    component: None,
                    net_name: Some(a_net),
                },
                ExtractedDeviceTerminal {
                    name: "B".to_string(),
                    component: None,
                    net_name: Some(b_net),
                },
            ],
            bounds: poly.bounds,
            width: poly.bounds.width().min(poly.bounds.height()),
            length: poly.bounds.width().max(poly.bounds.height()),
            occurrences: vec![poly.occurrence.clone()],
        });
    }
    extract_mim_capacitor_devices(document, components, shape_to_component, &mut devices);
    devices
}

pub(crate) fn extract_mim_capacitor_devices(
    document: &Document,
    components: &[NetComponent],
    shape_to_component: &BTreeMap<ShapeOccurrenceId, usize>,
    devices: &mut Vec<ExtractedDevice>,
) {
    let Some(metal1_layer) = document.layer_by_process(ProcessLayer::Metal1) else {
        return;
    };
    let Some(metal2_layer) = document.layer_by_process(ProcessLayer::Metal2) else {
        return;
    };
    let via_layer = document.layer_by_process(ProcessLayer::Via1);
    let (metal1_shapes, metal2_shapes, via_shapes, labels) =
        materialize_capacitor_shapes(document, metal1_layer, metal2_layer, via_layer);
    for lower in &metal1_shapes {
        for upper in &metal2_shapes {
            let Some(overlap) = lower.bounds.intersection(upper.bounds) else {
                continue;
            };
            if overlap.width() <= 0 || overlap.height() <= 0 {
                continue;
            }
            let has_via = via_shapes.iter().any(|via| {
                via.bounds
                    .intersection(overlap)
                    .is_some_and(|via_overlap| via_overlap.width() > 0 && via_overlap.height() > 0)
            });
            if has_via {
                continue;
            }
            let lower_component = shape_to_component.get(&lower.occurrence).copied();
            let upper_component = shape_to_component.get(&upper.occurrence).copied();
            let lower_terminal = extracted_terminal_for_region(
                "A",
                lower_component,
                components,
                &labels,
                lower.layer,
                overlap,
            );
            let upper_terminal = extracted_terminal_for_region(
                "B",
                upper_component,
                components,
                &labels,
                upper.layer,
                overlap,
            );
            let Some(lower_net) = lower_terminal.net_name.as_deref() else {
                continue;
            };
            let Some(upper_net) = upper_terminal.net_name.as_deref() else {
                continue;
            };
            if lower_net == upper_net {
                continue;
            }
            devices.push(ExtractedDevice {
                id: devices.len() + 1,
                kind: "capacitor".to_string(),
                model: "mimcap".to_string(),
                terminals: vec![lower_terminal, upper_terminal],
                bounds: overlap,
                width: overlap.width().min(overlap.height()),
                length: overlap.width().max(overlap.height()),
                occurrences: vec![lower.occurrence.clone(), upper.occurrence.clone()],
            });
        }
    }
}

pub(crate) fn materialize_device_shapes(
    document: &Document,
    diffusion_layer: LayerId,
    poly_layer: LayerId,
) -> (
    Vec<DeviceShape>,
    Vec<DeviceShape>,
    Vec<DeviceShape>,
    Vec<LabelShape>,
) {
    let mut diffusion_shapes = Vec::new();
    let mut poly_shapes = Vec::new();
    let mut body_shapes = Vec::new();
    let mut labels = Vec::new();
    for flattened in document.flattened_shapes() {
        let shape = flattened.transformed_shape();
        match &shape.kind {
            ShapeKind::Label { position, text } => {
                let text = text.trim();
                if !text.is_empty() {
                    labels.push(LabelShape {
                        occurrence: flattened.id,
                        layer: shape.layer,
                        text: text.to_string(),
                        position: *position,
                    });
                }
            }
            ShapeKind::Measurement { .. } => {}
            _ if shape.layer == diffusion_layer
                || shape.layer == poly_layer
                || is_mos_body_layer(document, shape.layer) =>
            {
                let bounds = shape.kind.bounds();
                if bounds.width() <= 0 || bounds.height() <= 0 {
                    continue;
                }
                let device_shape = DeviceShape {
                    occurrence: flattened.id,
                    layer: shape.layer,
                    bounds,
                };
                if shape.layer == diffusion_layer {
                    diffusion_shapes.push(device_shape);
                } else if shape.layer == poly_layer {
                    poly_shapes.push(device_shape);
                } else {
                    body_shapes.push(device_shape);
                }
            }
            _ => {}
        }
    }
    (diffusion_shapes, poly_shapes, body_shapes, labels)
}

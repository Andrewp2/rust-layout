#![allow(unused_imports)]
use super::*;

pub(crate) fn materialize_capacitor_shapes(
    document: &Document,
    lower_layer: LayerId,
    upper_layer: LayerId,
    via_layer: Option<LayerId>,
) -> (
    Vec<DeviceShape>,
    Vec<DeviceShape>,
    Vec<DeviceShape>,
    Vec<LabelShape>,
) {
    let mut lower_shapes = Vec::new();
    let mut upper_shapes = Vec::new();
    let mut via_shapes = Vec::new();
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
            _ if shape.layer == lower_layer
                || shape.layer == upper_layer
                || via_layer == Some(shape.layer) =>
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
                if shape.layer == lower_layer {
                    lower_shapes.push(device_shape);
                } else if shape.layer == upper_layer {
                    upper_shapes.push(device_shape);
                } else {
                    via_shapes.push(device_shape);
                }
            }
            _ => {}
        }
    }
    (lower_shapes, upper_shapes, via_shapes, labels)
}

pub(crate) fn extracted_terminal_for_component(
    name: &str,
    component: Option<usize>,
    components: &[NetComponent],
) -> ExtractedDeviceTerminal {
    ExtractedDeviceTerminal {
        name: name.to_string(),
        component,
        net_name: component
            .and_then(|component_id| components.get(component_id.saturating_sub(1)))
            .and_then(|component| component.net_name.clone()),
    }
}

pub(crate) fn extracted_terminal_for_region(
    name: &str,
    component: Option<usize>,
    components: &[NetComponent],
    labels: &[LabelShape],
    preferred_layer: LayerId,
    region: Rect,
) -> ExtractedDeviceTerminal {
    let mut terminal = extracted_terminal_for_component(name, component, components);
    if let Some(net_name) = label_net_for_device_shape(labels, preferred_layer, region) {
        terminal.net_name = Some(net_name);
    }
    terminal
}

pub(crate) fn mos_body_terminal_for_channel(
    body_shapes: &[DeviceShape],
    labels: &[LabelShape],
    channel: Rect,
) -> (ExtractedDeviceTerminal, Option<ShapeOccurrenceId>) {
    let body_shape = body_shapes
        .iter()
        .filter_map(|shape| {
            let overlap = shape.bounds.intersection(channel)?;
            (overlap.width() > 0 && overlap.height() > 0).then_some((shape, overlap.area()))
        })
        .max_by_key(|(_, area)| *area)
        .map(|(shape, _)| shape);
    let Some(body_shape) = body_shape else {
        return (
            ExtractedDeviceTerminal {
                name: "B".to_string(),
                component: None,
                net_name: Some("0".to_string()),
            },
            None,
        );
    };
    let net_name = label_net_for_preferred_layer(labels, body_shape.layer, body_shape.bounds)
        .or_else(|| label_net_for_preferred_layer(labels, body_shape.layer, channel))
        .or_else(|| Some("0".to_string()));
    (
        ExtractedDeviceTerminal {
            name: "B".to_string(),
            component: None,
            net_name,
        },
        Some(body_shape.occurrence.clone()),
    )
}

pub(crate) fn is_mos_body_layer(document: &Document, layer_id: LayerId) -> bool {
    let Some(layer) = document.layer(layer_id) else {
        return false;
    };
    let name = normalize_layer_ref(&layer.name);
    let purpose = normalize_layer_ref(&layer.purpose);
    ["well", "substrate", "bulk", "body"]
        .iter()
        .any(|needle| name.contains(needle) || purpose.contains(needle))
}

pub(crate) fn diffusion_terminal_regions(diffusion: Rect, channel: Rect) -> (Rect, Rect) {
    if channel.width() <= channel.height() {
        (
            Rect::new(
                diffusion.min,
                Point::new(channel.min.x.max(diffusion.min.x), diffusion.max.y),
            ),
            Rect::new(
                Point::new(channel.max.x.min(diffusion.max.x), diffusion.min.y),
                diffusion.max,
            ),
        )
    } else {
        (
            Rect::new(
                diffusion.min,
                Point::new(diffusion.max.x, channel.min.y.max(diffusion.min.y)),
            ),
            Rect::new(
                Point::new(diffusion.min.x, channel.max.y.min(diffusion.max.y)),
                diffusion.max,
            ),
        )
    }
}

pub(crate) fn resistor_terminal_regions(bounds: Rect) -> (Rect, Rect) {
    let center = bounds.center();
    if bounds.width() >= bounds.height() {
        (
            Rect::new(bounds.min, Point::new(center.x, bounds.max.y)),
            Rect::new(Point::new(center.x, bounds.min.y), bounds.max),
        )
    } else {
        (
            Rect::new(bounds.min, Point::new(bounds.max.x, center.y)),
            Rect::new(Point::new(bounds.min.x, center.y), bounds.max),
        )
    }
}

pub(crate) fn label_net_for_device_shape(
    labels: &[LabelShape],
    preferred_layer: LayerId,
    bounds: Rect,
) -> Option<String> {
    labels
        .iter()
        .filter(|label| label.layer == preferred_layer && bounds.contains_point(label.position))
        .chain(
            labels
                .iter()
                .filter(|label| bounds.contains_point(label.position)),
        )
        .map(|label| normalize_net_name(&label.text))
        .find(|name| !name.is_empty())
}

pub(crate) fn label_net_for_preferred_layer(
    labels: &[LabelShape],
    preferred_layer: LayerId,
    bounds: Rect,
) -> Option<String> {
    labels
        .iter()
        .filter(|label| label.layer == preferred_layer && bounds.contains_point(label.position))
        .map(|label| normalize_net_name(&label.text))
        .find(|name| !name.is_empty())
}

pub(crate) fn connectivity_layers(
    document: &Document,
    technology: &TechnologyFile,
) -> Result<(ConnectivityLayerSet, ConnectivityLayerPairs), TechnologyError> {
    let mut conductive_layers = BTreeSet::new();
    let mut connected_pairs = BTreeSet::new();
    for connection in &technology.connectivity {
        let from = document_layer_id(document, &connection.from)?;
        let through = document_layer_id(document, &connection.through)?;
        let to = document_layer_id(document, &connection.to)?;
        conductive_layers.insert(from);
        conductive_layers.insert(through);
        conductive_layers.insert(to);
        insert_layer_pair(&mut connected_pairs, from, through);
        insert_layer_pair(&mut connected_pairs, through, to);
    }
    Ok((conductive_layers, connected_pairs))
}

pub(crate) fn materialize_shapes(
    document: &Document,
    conductive_layers: &BTreeSet<LayerId>,
) -> (Vec<ConnectiveShape>, Vec<LabelShape>) {
    let mut connective_shapes = Vec::new();
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
            ShapeKind::Via { lower, upper, .. } => {
                let mut layers = BTreeSet::new();
                layers.insert(shape.layer);
                layers.insert(*lower);
                layers.insert(*upper);
                connective_shapes.push(ConnectiveShape {
                    index: connective_shapes.len(),
                    occurrence: flattened.id,
                    bounds: shape.kind.bounds(),
                    shape,
                    layers,
                });
            }
            _ if conductive_layers.contains(&shape.layer) => {
                let mut layers = BTreeSet::new();
                layers.insert(shape.layer);
                connective_shapes.push(ConnectiveShape {
                    index: connective_shapes.len(),
                    occurrence: flattened.id,
                    bounds: shape.kind.bounds(),
                    shape,
                    layers,
                });
            }
            _ => {}
        }
    }
    (connective_shapes, labels)
}

pub(crate) fn assign_labels(
    components: &mut [NetComponent],
    connective_shapes: &[ConnectiveShape],
    labels: &[LabelShape],
    union_find: &mut UnionFind,
    root_to_component: &BTreeMap<usize, usize>,
) {
    for label in labels {
        let mut component_ids = BTreeSet::new();
        let preferred = connective_shapes
            .iter()
            .filter(|shape| {
                shape.layers.contains(&label.layer) && shape.bounds.contains_point(label.position)
            })
            .map(|shape| shape.index)
            .collect::<Vec<_>>();
        let candidates: Vec<_> = if preferred.is_empty() {
            connective_shapes
                .iter()
                .filter(|shape| shape.bounds.contains_point(label.position))
                .map(|shape| shape.index)
                .collect()
        } else {
            preferred
        };
        for index in candidates {
            let root = union_find.find(index);
            if let Some(component) = root_to_component.get(&root) {
                component_ids.insert(*component);
            }
        }
        for component_id in component_ids {
            if let Some(component) = components.get_mut(component_id - 1) {
                component.labels.push(NetLabel {
                    occurrence: label.occurrence.clone(),
                    text: label.text.clone(),
                    position: label.position,
                });
            }
        }
    }
}

pub(crate) fn summarize_component_names(components: &mut [NetComponent]) -> Vec<NetShort> {
    let mut shorts = Vec::new();
    for component in components {
        let label_names = component
            .labels
            .iter()
            .map(|label| normalize_net_name(&label.text))
            .filter(|name| !name.is_empty())
            .collect::<BTreeSet<_>>();
        let explicit_nets = component
            .explicit_nets
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();

        if label_names.len() == 1 {
            component.net_name = label_names.iter().next().cloned();
        } else if label_names.is_empty() && explicit_nets.len() == 1 {
            component.net_name = explicit_nets.iter().next().map(|net| net_label(*net));
        }
        if explicit_nets.len() == 1 {
            component.net_id = explicit_nets.iter().next().copied();
        }

        if label_names.len() > 1 || explicit_nets.len() > 1 {
            let mut names = label_names.into_iter().collect::<Vec<_>>();
            names.extend(explicit_nets.into_iter().map(net_label));
            names.sort();
            names.dedup();
            shorts.push(NetShort {
                component: component.id,
                names,
                bounds: component.bounds,
            });
        }
    }
    shorts
}

pub(crate) fn find_open_nets(components: &[NetComponent]) -> Vec<NetOpen> {
    let mut by_name = BTreeMap::<String, Vec<&NetComponent>>::new();
    for component in components {
        let names = component
            .labels
            .iter()
            .map(|label| normalize_net_name(&label.text))
            .filter(|name| !name.is_empty())
            .chain(component.explicit_nets.iter().copied().map(net_label))
            .collect::<BTreeSet<_>>();
        for name in names {
            by_name.entry(name).or_default().push(component);
        }
    }

    by_name
        .into_iter()
        .filter_map(|(name, components)| {
            if components.len() <= 1 {
                return None;
            }
            let mut bounds = components[0].bounds;
            let ids = components
                .iter()
                .map(|component| {
                    bounds = bounds.union(component.bounds);
                    component.id
                })
                .collect();
            Some(NetOpen {
                name,
                components: ids,
                bounds,
            })
        })
        .collect()
}

pub(crate) fn layers_are_connected(
    left: &BTreeSet<LayerId>,
    right: &BTreeSet<LayerId>,
    connected_pairs: &BTreeSet<(LayerId, LayerId)>,
) -> bool {
    for left in left {
        for right in right {
            if left == right || connected_pairs.contains(&ordered_pair(*left, *right)) {
                return true;
            }
        }
    }
    false
}

pub(crate) fn document_layer_id(
    document: &Document,
    reference: &str,
) -> Result<LayerId, TechnologyError> {
    let normalized = normalize_layer_ref(reference);
    document
        .layers
        .values()
        .find(|layer| normalize_layer_ref(&layer.name) == normalized)
        .map(|layer| layer.id)
        .ok_or_else(|| {
            TechnologyError::Invalid(format!(
                "technology connectivity references missing document layer {reference:?}"
            ))
        })
}

pub(crate) fn insert_layer_pair(
    pairs: &mut BTreeSet<(LayerId, LayerId)>,
    left: LayerId,
    right: LayerId,
) {
    pairs.insert(ordered_pair(left, right));
}

pub(crate) fn ordered_pair(left: LayerId, right: LayerId) -> (LayerId, LayerId) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

pub(crate) fn normalize_layer_ref(reference: &str) -> String {
    reference
        .trim()
        .chars()
        .filter(|character| *character != '_' && *character != '-' && !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

pub(crate) fn normalize_net_name(name: &str) -> String {
    name.trim().to_ascii_uppercase()
}

pub(crate) fn net_label(net: NetId) -> String {
    format!("NET{}", net.0)
}

pub(crate) fn spice_component_net_name(component: &NetComponent) -> (String, bool) {
    if let Some(name) = component
        .net_name
        .as_ref()
        .map(|name| sanitize_spice_identifier(name, "NET"))
        .filter(|name| !name.is_empty())
    {
        (name, false)
    } else {
        (format!("N_{}", component.id), true)
    }
}

pub(crate) fn spice_subckt_pins(
    component_nets: &BTreeMap<usize, String>,
    devices: &[ExtractedDevice],
) -> Vec<String> {
    let mut pins = component_nets.values().cloned().collect::<Vec<_>>();
    for device in devices {
        for terminal in &device.terminals {
            let net = spice_terminal_net_name(terminal, component_nets, device.id);
            if net != "0" && !pins.contains(&net) {
                pins.push(net);
            }
        }
    }
    pins
}

pub(crate) fn spice_device_terminal_net(
    device: &ExtractedDevice,
    terminal_name: &str,
    component_nets: &BTreeMap<usize, String>,
) -> String {
    device
        .terminals
        .iter()
        .find(|terminal| terminal.name.eq_ignore_ascii_case(terminal_name))
        .map(|terminal| spice_terminal_net_name(terminal, component_nets, device.id))
        .unwrap_or_else(|| format!("N_DEV{}_{}", device.id, terminal_name))
}

pub(crate) fn spice_terminal_net_name(
    terminal: &ExtractedDeviceTerminal,
    component_nets: &BTreeMap<usize, String>,
    device_id: usize,
) -> String {
    if let Some(name) = terminal
        .net_name
        .as_deref()
        .map(|name| sanitize_spice_identifier(name, "NET"))
        .filter(|name| !name.is_empty())
    {
        return name;
    }
    if let Some(component) = terminal.component
        && let Some(net) = component_nets.get(&component)
    {
        return net.clone();
    }
    sanitize_spice_identifier(&format!("DEV{}_{}", device_id, terminal.name), "NET")
}

pub(crate) fn sanitize_spice_identifier(value: &str, fallback: &str) -> String {
    let mut output = String::new();
    let mut previous_separator = false;
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':' | '.') {
            output.push(ch.to_ascii_uppercase());
            previous_separator = false;
        } else if !previous_separator {
            output.push('_');
            previous_separator = true;
        }
    }
    let output = output.trim_matches('_').to_string();
    if output.is_empty() {
        fallback.to_string()
    } else if output == "0" {
        output
    } else if output
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
    {
        output
    } else {
        format!("N_{output}")
    }
}

pub(crate) fn spice_comment(value: &str) -> String {
    let mut output = value
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect::<String>();
    output = output.split_whitespace().collect::<Vec<_>>().join(" ");
    if output.is_empty() {
        "-".to_string()
    } else {
        output
    }
}

pub(crate) fn spice_rect(rect: Rect) -> String {
    format!(
        "({},{})-({},{})",
        rect.min.x, rect.min.y, rect.max.x, rect.max.y
    )
}

pub(crate) fn write_spice_subckt_line(output: &mut String, circuit_name: &str, pins: &[String]) {
    output.push_str(&format!(".subckt {circuit_name}"));
    for pin in pins {
        output.push(' ');
        output.push_str(pin);
    }
    output.push('\n');
}

pub(crate) fn spice_logical_lines(text: &str) -> Vec<(usize, String)> {
    let mut lines = Vec::new();
    let mut current = None::<(usize, String)>;
    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim_end();
        let trimmed = line.trim_start();
        if let Some(continued) = trimmed.strip_prefix('+') {
            if let Some((_, current_line)) = current.as_mut() {
                current_line.push(' ');
                current_line.push_str(continued.trim_start());
            } else {
                current = Some((line_number, continued.trim_start().to_string()));
            }
        } else {
            if let Some(line) = current.take() {
                lines.push(line);
            }
            current = Some((line_number, line.to_string()));
        }
    }
    if let Some(line) = current {
        lines.push(line);
    }
    lines
}

pub(crate) fn strip_spice_inline_comment(line: &str) -> &str {
    let mut end = line.len();
    for marker in [';', '$'] {
        if let Some(index) = line.find(marker) {
            end = end.min(index);
        }
    }
    &line[..end]
}

pub(crate) fn spice_device_terminal_count(device_name: &str, value_count: usize) -> usize {
    let prefix = device_name
        .chars()
        .next()
        .map(|character| character.to_ascii_uppercase())
        .unwrap_or('X');
    let expected = match prefix {
        'M' => 4,
        'Q' | 'J' => 3,
        'D' | 'R' | 'C' | 'L' | 'V' | 'I' | 'F' | 'H' => 2,
        'E' | 'G' => 4,
        'X' => value_count.saturating_sub(1),
        _ => value_count.saturating_sub(1).max(1),
    };
    expected.min(value_count)
}

pub(crate) fn parse_spice_device_parameter(token: &str) -> Option<(String, String)> {
    let (key, value) = token.split_once('=')?;
    let key = sanitize_spice_identifier(key, "PARAM");
    let value = sanitize_spice_parameter_value(value);
    (!key.is_empty() && !value.is_empty()).then_some((key, value))
}

pub(crate) fn sanitize_spice_parameter_value(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '+' | '-'))
        .map(|ch| ch.to_ascii_uppercase())
        .collect()
}

pub(crate) fn dedupe_spice_identifiers(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

pub(crate) fn validate_component_names(
    component: &NetComponent,
    findings: &mut Vec<ConnectivityValidationFinding>,
) {
    let mut explicit_nets = BTreeSet::new();
    for net in &component.explicit_nets {
        if !explicit_nets.insert(*net) {
            findings.push(ConnectivityValidationFinding::error(format!(
                "connectivity component {} repeats explicit net {:?}",
                component.id, net
            )));
        }
    }

    if let Some(net_id) = component.net_id
        && !explicit_nets.contains(&net_id)
    {
        findings.push(ConnectivityValidationFinding::error(format!(
            "connectivity component {} net_id {:?} is not present in explicit_nets",
            component.id, net_id
        )));
    }

    let mut derived_names = BTreeSet::new();
    for label in &component.labels {
        let normalized = normalize_net_name(&label.text);
        if normalized.is_empty() {
            findings.push(ConnectivityValidationFinding::error(format!(
                "connectivity component {} has an empty net label",
                component.id
            )));
        } else {
            derived_names.insert(normalized);
        }
    }
    derived_names.extend(explicit_nets.into_iter().map(net_label));

    if let Some(net_name) = &component.net_name {
        let normalized = normalize_net_name(net_name);
        if normalized.is_empty() {
            findings.push(ConnectivityValidationFinding::error(format!(
                "connectivity component {} has an empty net_name",
                component.id
            )));
        } else if derived_names.is_empty() {
            findings.push(ConnectivityValidationFinding::error(format!(
                "connectivity component {} net_name {:?} has no label or explicit net source",
                component.id, net_name
            )));
        } else if !derived_names.contains(&normalized) {
            findings.push(ConnectivityValidationFinding::error(format!(
                "connectivity component {} net_name {:?} is not derived from labels or explicit nets",
                component.id, net_name
            )));
        }
    }
}

pub(crate) fn validate_short(
    short: &NetShort,
    component_ids: &BTreeSet<usize>,
    issue_keys: &mut BTreeSet<String>,
    findings: &mut Vec<ConnectivityValidationFinding>,
) {
    if !component_ids.contains(&short.component) {
        findings.push(ConnectivityValidationFinding::error(format!(
            "connectivity short {:?} references missing component {}",
            short.names, short.component
        )));
    }

    let mut normalized_names = BTreeSet::new();
    for name in &short.names {
        let normalized = normalize_net_name(name);
        if normalized.is_empty() {
            findings.push(ConnectivityValidationFinding::error(
                "connectivity short has an empty net name",
            ));
        } else if normalized != *name {
            findings.push(ConnectivityValidationFinding::warning(format!(
                "connectivity short net name {name:?} is not normalized"
            )));
        }
        if !normalized.is_empty() && !normalized_names.insert(normalized) {
            findings.push(ConnectivityValidationFinding::error(format!(
                "connectivity short repeats net name {name:?}"
            )));
        }
    }
    if normalized_names.len() < 2 {
        findings.push(ConnectivityValidationFinding::error(format!(
            "connectivity short on component {} has fewer than two distinct net names",
            short.component
        )));
    }

    validate_issue_key("short", short.stable_key(), issue_keys, findings);
}

pub(crate) fn validate_open(
    open: &NetOpen,
    component_ids: &BTreeSet<usize>,
    issue_keys: &mut BTreeSet<String>,
    findings: &mut Vec<ConnectivityValidationFinding>,
) {
    let normalized_name = normalize_net_name(&open.name);
    if normalized_name.is_empty() {
        findings.push(ConnectivityValidationFinding::error(
            "connectivity open has an empty net name",
        ));
    } else if normalized_name != open.name {
        findings.push(ConnectivityValidationFinding::warning(format!(
            "connectivity open net name {:?} is not normalized",
            open.name
        )));
    }

    let mut open_components = BTreeSet::new();
    for component_id in &open.components {
        if !component_ids.contains(component_id) {
            findings.push(ConnectivityValidationFinding::error(format!(
                "connectivity open {:?} references missing component {}",
                open.name, component_id
            )));
        }
        if !open_components.insert(*component_id) {
            findings.push(ConnectivityValidationFinding::error(format!(
                "connectivity open {:?} repeats component {}",
                open.name, component_id
            )));
        }
    }
    if open_components.len() < 2 {
        findings.push(ConnectivityValidationFinding::error(format!(
            "connectivity open {:?} has fewer than two distinct components",
            open.name
        )));
    }

    validate_issue_key("open", open.stable_key(), issue_keys, findings);
}

pub(crate) fn validate_issue_key(
    kind: &str,
    key: String,
    issue_keys: &mut BTreeSet<String>,
    findings: &mut Vec<ConnectivityValidationFinding>,
) {
    if key.trim().is_empty() {
        findings.push(ConnectivityValidationFinding::error(format!(
            "connectivity {kind} produced an empty stable issue key"
        )));
    } else if !issue_keys.insert(key.clone()) {
        findings.push(ConnectivityValidationFinding::error(format!(
            "connectivity stable issue key {key:?} is duplicated"
        )));
    }
}

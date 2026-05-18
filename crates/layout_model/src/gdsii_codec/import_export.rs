#![allow(unused_imports)]
use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_gds_path_style(
    points: &[Point],
    width: Coord,
    pathtype: u16,
    begin_extension: Option<Coord>,
    end_extension: Option<Coord>,
    report: &mut GdsImportReport,
    layer: u16,
    datatype: u16,
    layer_id: LayerId,
) -> Vec<Point> {
    let half_width = width / 2;
    let (begin, end) = match pathtype {
        0 => {
            report_ignored_gds_path_extensions(
                begin_extension,
                end_extension,
                report,
                layer,
                datatype,
                layer_id,
                pathtype,
            );
            (0, 0)
        }
        1 => {
            report.warnings.push(GdsImportWarning {
                kind: GdsImportWarningKind::UnsupportedPathStyle,
                gds_layer: Some(layer),
                gds_type: Some(datatype),
                is_text: Some(false),
                layer_id: Some(layer_id),
                message: "GDS PATH round caps imported as square half-width extensions".to_string(),
            });
            (half_width, half_width)
        }
        2 => {
            report_ignored_gds_path_extensions(
                begin_extension,
                end_extension,
                report,
                layer,
                datatype,
                layer_id,
                pathtype,
            );
            (half_width, half_width)
        }
        4 => (begin_extension.unwrap_or(0), end_extension.unwrap_or(0)),
        _ => {
            report.warnings.push(GdsImportWarning {
                kind: GdsImportWarningKind::UnsupportedPathStyle,
                gds_layer: Some(layer),
                gds_type: Some(datatype),
                is_text: Some(false),
                layer_id: Some(layer_id),
                message: format!(
                    "unsupported GDS PATH pathtype {pathtype}; imported without end extension"
                ),
            });
            (0, 0)
        }
    };
    extend_path_endpoints(points, begin, end)
}

pub(crate) fn report_ignored_gds_path_extensions(
    begin_extension: Option<Coord>,
    end_extension: Option<Coord>,
    report: &mut GdsImportReport,
    layer: u16,
    datatype: u16,
    layer_id: LayerId,
    pathtype: u16,
) {
    if begin_extension.is_none() && end_extension.is_none() {
        return;
    }
    report.warnings.push(GdsImportWarning {
        kind: GdsImportWarningKind::UnsupportedPathStyle,
        gds_layer: Some(layer),
        gds_type: Some(datatype),
        is_text: Some(false),
        layer_id: Some(layer_id),
        message: format!(
            "GDS PATH extension records are only applied for pathtype 4; pathtype {pathtype} imported without custom extensions"
        ),
    });
}

pub(crate) fn extend_path_endpoints(
    points: &[Point],
    begin_extension: Coord,
    end_extension: Coord,
) -> Vec<Point> {
    let mut extended = points.to_vec();
    if extended.len() < 2 {
        return extended;
    }
    if begin_extension != 0 {
        let first = extended[0];
        let next = extended[1];
        if let Some(point) = point_extended_along_segment(next, first, begin_extension) {
            extended[0] = point;
        }
    }
    if end_extension != 0 {
        let last_index = extended.len() - 1;
        let previous = extended[last_index - 1];
        let last = extended[last_index];
        if let Some(point) = point_extended_along_segment(previous, last, end_extension) {
            extended[last_index] = point;
        }
    }
    extended
}

pub(crate) fn point_extended_along_segment(a: Point, b: Point, extension: Coord) -> Option<Point> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    if dx == 0 && dy == 0 {
        return None;
    }
    if dy == 0 {
        return Some(Point::new(b.x + extension * dx.signum(), b.y));
    }
    if dx == 0 {
        return Some(Point::new(b.x, b.y + extension * dy.signum()));
    }
    let length = ((dx as f64).powi(2) + (dy as f64).powi(2)).sqrt();
    let scale = extension as f64 / length;
    Some(Point::new(
        coord_from_f64_rounded(b.x as f64 + dx as f64 * scale),
        coord_from_f64_rounded(b.y as f64 + dy as f64 * scale),
    ))
}

pub(crate) fn coord_from_f64_rounded(value: f64) -> Coord {
    let rounded = value.round();
    if rounded > i64::MAX as f64 {
        i64::MAX
    } else if rounded < i64::MIN as f64 {
        i64::MIN
    } else {
        rounded as Coord
    }
}

pub(crate) fn report_duplicate_structure_name_warnings(
    library: &ParsedLibrary,
    report: &mut GdsImportReport,
) {
    let mut counts = BTreeMap::<&str, usize>::new();
    for structure in &library.structures {
        *counts.entry(structure.name.as_str()).or_default() += 1;
    }
    for (name, count) in counts {
        if count < 2 {
            continue;
        }
        report.warnings.push(GdsImportWarning {
            kind: GdsImportWarningKind::DuplicateStructureName,
            gds_layer: None,
            gds_type: None,
            is_text: None,
            layer_id: None,
            message: format!(
                "GDS structure name {name:?} appears {count} times; duplicate definitions import into one Glassworks cell and SREF/AREF references are ambiguous"
            ),
        });
    }
}

pub(crate) fn record_import_layer_mapping(
    resolved_layer_mappings: &mut BTreeMap<GdsImportMappingKey, BTreeSet<LayerId>>,
    gds_layer: u16,
    gds_type: u16,
    is_text: bool,
    layer_id: LayerId,
) {
    resolved_layer_mappings
        .entry(GdsImportMappingKey {
            gds_layer,
            gds_type,
            is_text,
        })
        .or_default()
        .insert(layer_id);
}

pub(crate) fn report_split_import_layer_mapping_warnings(
    resolved_layer_mappings: &BTreeMap<GdsImportMappingKey, BTreeSet<LayerId>>,
    report: &mut GdsImportReport,
) {
    let mut by_gds_layer: BTreeMap<u16, BTreeMap<LayerId, Vec<GdsImportMappingKey>>> =
        BTreeMap::new();
    for (mapping, layer_ids) in resolved_layer_mappings {
        for layer_id in layer_ids {
            by_gds_layer
                .entry(mapping.gds_layer)
                .or_default()
                .entry(*layer_id)
                .or_default()
                .push(*mapping);
        }
    }
    for (gds_layer, layer_groups) in by_gds_layer {
        if layer_groups.len() < 2 {
            continue;
        }
        let details = layer_groups
            .iter()
            .map(|(layer_id, mappings)| {
                let mapping_labels = mappings
                    .iter()
                    .map(format_import_mapping_label)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("layer {} from {mapping_labels}", layer_id.0)
            })
            .collect::<Vec<_>>()
            .join("; ");
        report.warnings.push(GdsImportWarning {
            kind: GdsImportWarningKind::SplitIncomingLayerMapping,
            gds_layer: Some(gds_layer),
            gds_type: None,
            is_text: None,
            layer_id: None,
            message: format!(
                "incoming GDS layer {gds_layer} resolved to multiple Glassworks layers: {details}"
            ),
        });
    }
}

pub(crate) fn format_import_mapping_label(mapping: &GdsImportMappingKey) -> String {
    if mapping.is_text {
        format!("TEXTTYPE {}", mapping.gds_type)
    } else {
        format!("DATATYPE {}", mapping.gds_type)
    }
}

pub(crate) fn parse_gdsii(bytes: &[u8]) -> Result<ParsedLibrary, GdsError> {
    let mut reader = GdsReader::new(bytes);
    let mut library = ParsedLibrary {
        name: "gds_import".to_string(),
        dbu_per_micron: DBU_PER_MICRON,
        warnings: Vec::new(),
        structures: Vec::new(),
    };
    let mut structure: Option<ParsedStructure> = None;
    let mut element: Option<ElementBuilder> = None;
    let mut saw_endlib = false;

    while let Some(record) = reader.next_record()? {
        match record.record_type {
            LIBNAME => library.name = record.as_ascii()?,
            UNITS => {
                let units = record.as_real8()?;
                if let Some(user_units_per_dbu) = units.first().copied()
                    && user_units_per_dbu.is_finite()
                    && user_units_per_dbu > 0.0
                {
                    let parsed = (1.0 / user_units_per_dbu).round();
                    let effective = parsed.max(1.0) as Coord;
                    if parsed < 1.0 {
                        warn!(
                            parsed_dbu_per_micron = parsed,
                            effective_dbu_per_micron = effective,
                            "GDS UNITS yielded unsupported dbu_per_micron; using minimum 1"
                        );
                        library.warnings.push(GdsImportWarning {
                            kind: GdsImportWarningKind::NormalizedUnits,
                            gds_layer: None,
                            gds_type: None,
                            is_text: None,
                            layer_id: None,
                            message: format!(
                                "GDS UNITS resolved to unsupported dbu_per_micron {parsed}; using minimum dbu_per_micron {effective}"
                            ),
                        });
                    }
                    library.dbu_per_micron = effective;
                } else {
                    library.warnings.push(GdsImportWarning {
                        kind: GdsImportWarningKind::NormalizedUnits,
                        gds_layer: None,
                        gds_type: None,
                        is_text: None,
                        layer_id: None,
                        message: format!(
                            "GDS UNITS record is invalid; using default dbu_per_micron {}",
                            library.dbu_per_micron
                        ),
                    });
                }
            }
            BGNSTR => {
                if let Some(open_element) = &element {
                    return Err(GdsError::InvalidRecord(format!(
                        "BGNSTR before ENDEL for {} element",
                        open_element.kind.label()
                    )));
                }
                if structure.is_some() {
                    return Err(GdsError::InvalidRecord(
                        "BGNSTR before ENDSTR for previous structure".to_string(),
                    ));
                }
                structure = Some(ParsedStructure {
                    name: String::new(),
                    elements: Vec::new(),
                });
            }
            STRNAME => {
                if let Some(structure) = &mut structure {
                    structure.name = record.as_ascii()?;
                }
            }
            ENDSTR => {
                if let Some(open_element) = &element {
                    return Err(GdsError::InvalidRecord(format!(
                        "ENDSTR before ENDEL for {} element",
                        open_element.kind.label()
                    )));
                }
                let mut finished = structure
                    .take()
                    .ok_or_else(|| GdsError::InvalidRecord("ENDSTR without BGNSTR".to_string()))?;
                if finished.name.trim().is_empty() {
                    finished.name = format!("STRUCT_{}", library.structures.len() + 1);
                }
                library.structures.push(finished);
            }
            BOUNDARY => begin_gds_element(&structure, &mut element, ElementKind::Boundary)?,
            PATH => begin_gds_element(&structure, &mut element, ElementKind::Path)?,
            TEXT => begin_gds_element(&structure, &mut element, ElementKind::Text)?,
            SREF => begin_gds_element(&structure, &mut element, ElementKind::Sref)?,
            AREF => begin_gds_element(&structure, &mut element, ElementKind::Aref)?,
            TEXTNODE => begin_gds_element(
                &structure,
                &mut element,
                ElementKind::Unsupported("TEXTNODE"),
            )?,
            NODE => begin_gds_element(&structure, &mut element, ElementKind::Unsupported("NODE"))?,
            BOX => begin_gds_element(&structure, &mut element, ElementKind::Box)?,
            LAYER => {
                if let Some(element) = &mut element {
                    element.layer = Some(record.first_u16()?);
                }
            }
            DATATYPE | NODETYPE | BOXTYPE => {
                if let Some(element) = &mut element {
                    element.datatype = record.first_u16()?;
                }
            }
            TEXTTYPE => {
                if let Some(element) = &mut element {
                    element.texttype = record.first_u16()?;
                }
            }
            WIDTH => {
                if let Some(element) = &mut element {
                    element.width = Coord::from(record.first_i32()?);
                }
            }
            PATHTYPE => {
                if let Some(element) = &mut element {
                    element.pathtype = record.first_u16()?;
                }
            }
            BGNEXTN => {
                if let Some(element) = &mut element {
                    element.begin_extension = Some(Coord::from(record.first_i32()?));
                }
            }
            ENDEXTN => {
                if let Some(element) = &mut element {
                    element.end_extension = Some(Coord::from(record.first_i32()?));
                }
            }
            PROPATTR => {
                if let Some(element) = &mut element {
                    element.pending_property_attr = Some(record.first_u16()?);
                }
            }
            PROPVALUE => {
                if let Some(element) = &mut element {
                    if let Some(attr) = element.pending_property_attr {
                        element.properties.push(GdsProperty {
                            attr,
                            value: record.as_ascii()?,
                        });
                    }
                }
            }
            XY => {
                if let Some(element) = &mut element {
                    element.points = record.as_xy()?;
                }
            }
            SNAME => {
                if let Some(element) = &mut element {
                    element.name = Some(record.as_ascii()?);
                }
            }
            STRANS => {
                if let Some(element) = &mut element {
                    element.transform.reflect_x =
                        record.first_bit_array()? & GDS_STRANS_REFLECT_X != 0;
                }
            }
            MAG => {
                if let Some(element) = &mut element {
                    element.transform.magnification = Some(record.first_real8()?);
                }
            }
            ANGLE => {
                if let Some(element) = &mut element {
                    element.transform.angle_degrees = Some(record.first_real8()?);
                }
            }
            STRING => {
                if let Some(element) = &mut element {
                    element.text = Some(record.as_ascii()?);
                }
            }
            COLROW => {
                if let Some(element) = &mut element {
                    let values = record.as_u16s()?;
                    if values.len() >= 2 {
                        element.columns = values[0];
                        element.rows = values[1];
                    }
                }
            }
            ENDEL => {
                let finished = element
                    .take()
                    .ok_or_else(|| GdsError::InvalidRecord("ENDEL without element".to_string()))?
                    .finish()?;
                let structure = structure.as_mut().ok_or_else(|| {
                    GdsError::InvalidRecord("element outside structure".to_string())
                })?;
                structure.elements.push(finished);
            }
            ENDLIB => {
                if let Some(open_element) = &element {
                    return Err(GdsError::InvalidRecord(format!(
                        "ENDLIB before ENDEL for {} element",
                        open_element.kind.label()
                    )));
                }
                if structure.is_some() {
                    return Err(GdsError::InvalidRecord(
                        "ENDLIB before ENDSTR for open structure".to_string(),
                    ));
                }
                saw_endlib = true;
                break;
            }
            _ => {}
        }
    }
    if let Some(open_element) = &element {
        return Err(GdsError::InvalidRecord(format!(
            "unexpected EOF before ENDEL for {} element",
            open_element.kind.label()
        )));
    }
    if structure.is_some() {
        return Err(GdsError::InvalidRecord(
            "unexpected EOF before ENDSTR for open structure".to_string(),
        ));
    }
    if !saw_endlib {
        return Err(GdsError::InvalidRecord(
            "unexpected EOF before ENDLIB".to_string(),
        ));
    }
    Ok(library)
}

pub(crate) fn begin_gds_element(
    structure: &Option<ParsedStructure>,
    element: &mut Option<ElementBuilder>,
    kind: ElementKind,
) -> Result<(), GdsError> {
    if structure.is_none() {
        return Err(GdsError::InvalidRecord(format!(
            "{} element outside structure",
            kind.label()
        )));
    }
    if let Some(open_element) = element {
        return Err(GdsError::InvalidRecord(format!(
            "{} before ENDEL for {} element",
            kind.label(),
            open_element.kind.label()
        )));
    }
    *element = Some(ElementBuilder::new(kind));
    Ok(())
}

pub(crate) fn structure_names(document: &Document) -> BTreeMap<CellId, String> {
    let mut used = BTreeSet::new();
    let mut names = BTreeMap::new();
    for cell in document.cells.values() {
        let fallback = if cell.id == document.top_cell {
            "TOP"
        } else {
            "CELL"
        };
        let base = sanitize_gds_name(&cell.name, fallback);
        let mut candidate = base.clone();
        let mut suffix = 1;
        while !used.insert(candidate.clone()) {
            candidate = truncate_gds_name(&format!("{base}_{suffix}"));
            suffix += 1;
        }
        names.insert(cell.id, candidate);
    }
    names
}

pub(crate) fn infer_top_structure_name(library: &ParsedLibrary) -> Option<String> {
    let mut referenced = BTreeSet::new();
    for structure in &library.structures {
        for element in &structure.elements {
            match element {
                ParsedElement::Sref { name, .. } | ParsedElement::Aref { name, .. } => {
                    referenced.insert(name.clone());
                }
                ParsedElement::Boundary { .. }
                | ParsedElement::Box { .. }
                | ParsedElement::Path { .. }
                | ParsedElement::Text { .. }
                | ParsedElement::Unsupported { .. } => {}
            }
        }
    }
    library
        .structures
        .iter()
        .rev()
        .find(|structure| !referenced.contains(&structure.name))
        .or_else(|| library.structures.last())
        .map(|structure| structure.name.clone())
}

pub(crate) fn insert_shape_in_import_cell(
    document: &mut Document,
    cell: CellId,
    layer: LayerId,
    kind: ShapeKind,
    name: Option<String>,
    net: Option<NetId>,
    properties: BTreeMap<String, String>,
) -> ShapeId {
    let id = document.allocate_shape_id();
    let shape = Shape {
        id,
        layer,
        net,
        kind,
        name,
        properties,
    };
    if cell == document.top_cell {
        document.shapes.insert(id, shape);
    } else if let Some(cell) = document.cells.get_mut(&cell) {
        cell.shapes.insert(id, shape);
    }
    id
}

pub(crate) fn gds_shape_name_property(properties: &[GdsProperty]) -> Option<String> {
    properties
        .iter()
        .find(|property| property.attr == GLASSWORKS_GDS_SHAPE_NAME_PROP_ATTR)
        .map(|property| property.value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn gds_shape_net_property(properties: &[GdsProperty]) -> Option<NetId> {
    properties
        .iter()
        .find(|property| property.attr == GLASSWORKS_GDS_SHAPE_NET_PROP_ATTR)
        .and_then(|property| property.value.trim().parse::<u32>().ok())
        .map(NetId)
}

pub(crate) fn gds_shape_properties(properties: &[GdsProperty]) -> BTreeMap<String, String> {
    let mut shape_properties = BTreeMap::new();
    for property in properties {
        match property.attr {
            GLASSWORKS_GDS_SHAPE_NAME_PROP_ATTR | GLASSWORKS_GDS_SHAPE_NET_PROP_ATTR => {}
            GLASSWORKS_GDS_SHAPE_PROPERTY_PROP_ATTR => {
                if let Some((key, value)) = decode_glassworks_shape_property(&property.value) {
                    insert_shape_property(&mut shape_properties, key, value);
                } else {
                    insert_shape_property(
                        &mut shape_properties,
                        format!("gds.attr.{}", property.attr),
                        property.value.clone(),
                    );
                }
            }
            attr => {
                insert_shape_property(
                    &mut shape_properties,
                    format!("gds.attr.{attr}"),
                    property.value.clone(),
                );
            }
        }
    }
    shape_properties
}

pub(crate) fn gds_instance_properties(properties: &[GdsProperty]) -> BTreeMap<String, String> {
    let mut instance_properties = BTreeMap::new();
    for property in properties {
        if property.attr == GLASSWORKS_GDS_SHAPE_PROPERTY_PROP_ATTR {
            if let Some((key, value)) = decode_glassworks_shape_property(&property.value) {
                insert_shape_property(&mut instance_properties, key, value);
            } else {
                insert_shape_property(
                    &mut instance_properties,
                    format!("gds.attr.{}", property.attr),
                    property.value.clone(),
                );
            }
        } else {
            insert_shape_property(
                &mut instance_properties,
                format!("gds.attr.{}", property.attr),
                property.value.clone(),
            );
        }
    }
    instance_properties
}

pub(crate) fn decode_glassworks_shape_property(encoded: &str) -> Option<(String, String)> {
    let mut key = String::new();
    let mut value = String::new();
    let mut in_value = false;
    let mut escaped = false;
    for ch in encoded.chars() {
        if escaped {
            push_decoded_shape_property_char(if in_value { &mut value } else { &mut key }, ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '=' if !in_value => in_value = true,
            _ if in_value => value.push(ch),
            _ => key.push(ch),
        }
    }
    if escaped {
        if in_value {
            value.push('\\');
        } else {
            key.push('\\');
        }
    }
    if !in_value || key.trim().is_empty() {
        return None;
    }
    Some((key, value))
}

pub(crate) fn push_decoded_shape_property_char(target: &mut String, ch: char) {
    target.push(match ch {
        'n' => '\n',
        'r' => '\r',
        other => other,
    });
}

pub(crate) fn insert_shape_property(
    properties: &mut BTreeMap<String, String>,
    key: impl Into<String>,
    value: String,
) {
    let key = key.into();
    let key = key.trim();
    if key.is_empty() {
        return;
    }
    if !properties.contains_key(key) {
        properties.insert(key.to_string(), value);
        return;
    }
    for suffix in 2.. {
        let suffixed = format!("{key}.{suffix}");
        if !properties.contains_key(&suffixed) {
            properties.insert(suffixed, value);
            return;
        }
    }
}

pub(crate) fn insert_import_instance(
    document: &mut Document,
    parent: CellId,
    child: CellId,
    name: &str,
    transform: Transform,
    array: InstanceArray,
    properties: BTreeMap<String, String>,
) {
    let id = document.allocate_instance_id();
    let instance = CellInstance {
        id,
        name: Some(sanitize_gds_name(name, "inst").to_ascii_lowercase()),
        cell: child,
        transform,
        array,
        properties,
    };
    if let Some(parent) = document.cells.get_mut(&parent) {
        parent.instances.insert(id, instance);
    }
}

pub(crate) fn import_boundary_kind(points: &[Point]) -> ShapeKind {
    let mut points = points.to_vec();
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    if let Some(rect) = axis_aligned_rect(&points) {
        ShapeKind::Rectangle(rect)
    } else {
        ShapeKind::Polygon(Polygon::new(points))
    }
}

pub(crate) fn import_boundary_has_minimum_points(points: &[Point]) -> bool {
    let mut points = points.to_vec();
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    let distinct = points.iter().copied().collect::<BTreeSet<_>>();
    distinct.len() >= 3
}

pub(crate) fn axis_aligned_rect(points: &[Point]) -> Option<Rect> {
    if points.len() != 4 {
        return None;
    }
    let xs: BTreeSet<_> = points.iter().map(|point| point.x).collect();
    let ys: BTreeSet<_> = points.iter().map(|point| point.y).collect();
    if xs.len() != 2 || ys.len() != 2 {
        return None;
    }
    let min = Point::new(*xs.iter().next()?, *ys.iter().next()?);
    let max = Point::new(*xs.iter().next_back()?, *ys.iter().next_back()?);
    let expected: BTreeSet<_> = Rect::new(min, max).corners().into_iter().collect();
    let actual: BTreeSet<_> = points.iter().copied().collect();
    (expected == actual).then_some(Rect::new(min, max))
}

pub(crate) fn resolve_import_layer(
    document: &mut Document,
    technology: &TechnologyFile,
    generated_layers: &mut BTreeMap<(u16, u16, bool), LayerId>,
    report: &mut GdsImportReport,
    gds_layer: u16,
    gds_type: u16,
    is_text: bool,
) -> LayerId {
    let technology_layer = if is_text {
        technology.layer_for_gds_text(gds_layer, gds_type)
    } else {
        technology.layer_for_gds_geometry(gds_layer, gds_type)
    };
    if let Some(layer) = technology_layer.and_then(|layer| layer.id)
        && document.layers.contains_key(&layer)
    {
        return layer;
    }
    if let Some((id, _)) = document.layers.iter().find(|(_, layer)| {
        layer.gds_layer == Some(gds_layer)
            && if is_text {
                layer.gds_texttype == gds_type
            } else {
                layer.gds_datatype == gds_type
            }
    }) {
        return *id;
    }
    let key = (gds_layer, gds_type, is_text);
    if let Some(id) = generated_layers.get(&key) {
        return *id;
    }
    let name = if is_text {
        format!("gds_{gds_layer}_text_{gds_type}")
    } else {
        format!("gds_{gds_layer}_{gds_type}")
    };
    let id = document.create_layer(
        name,
        ProcessLayer::Annotation,
        generated_layer_color(gds_layer),
    );
    if let Some(layer) = document.layers.get_mut(&id) {
        layer.purpose = if is_text {
            format!("imported GDSII texttype {gds_type}")
        } else {
            format!("imported GDSII datatype {gds_type}")
        };
        layer.gds_layer = Some(gds_layer);
        if is_text {
            layer.gds_texttype = gds_type;
        } else {
            layer.gds_datatype = gds_type;
        }
    }
    generated_layers.insert(key, id);
    report.generated_layers.push(GdsGeneratedLayer {
        gds_layer,
        gds_type,
        is_text,
        layer_id: id,
        name: document
            .layers
            .get(&id)
            .map(|layer| layer.name.clone())
            .unwrap_or_else(|| format!("gds_{gds_layer}_{gds_type}")),
    });
    id
}

pub(crate) fn gds_mapping_for_export(
    document: &Document,
    technology: &TechnologyFile,
    ambiguous_mappings: &BTreeMap<GdsExportMappingKey, Vec<LayerId>>,
    cell_id: CellId,
    shape: &Shape,
    report: &mut GdsExportReport,
) -> Result<(u16, u16, u16), GdsError> {
    let layer_id = shape.layer;
    if let Some(mapping) = technology.gds_mapping_for_layer(layer_id) {
        report_ambiguous_gds_mapping_warnings(
            ambiguous_mappings,
            report,
            cell_id,
            shape,
            layer_id,
            mapping,
        );
        return Ok(mapping);
    }
    if let Some(layer) = document.layer(layer_id) {
        if let Some(gds_layer) = layer.gds_layer {
            let mapping = (gds_layer, layer.gds_datatype, layer.gds_texttype);
            report_ambiguous_gds_mapping_warnings(
                ambiguous_mappings,
                report,
                cell_id,
                shape,
                layer_id,
                mapping,
            );
            return Ok(mapping);
        }
        let gds_layer = u16::try_from(layer.id.0)
            .map_err(|_| GdsError::Unsupported(format!("layer id {} exceeds GDSII", layer.id.0)))?;
        let mapping = (gds_layer, 0, 0);
        report.warnings.push(GdsExportWarning {
            kind: GdsExportWarningKind::FallbackLayerMapping,
            cell_id,
            shape_id: Some(shape.id),
            instance_id: None,
            layer_id: Some(layer.id),
            message: format!(
                "layer {} has no technology or document GDS mapping; exported with layer id {}",
                layer.id.0, gds_layer
            ),
        });
        report_ambiguous_gds_mapping_warnings(
            ambiguous_mappings,
            report,
            cell_id,
            shape,
            layer_id,
            mapping,
        );
        return Ok(mapping);
    }
    let gds_layer = u16::try_from(layer_id.0)
        .map_err(|_| GdsError::Unsupported(format!("layer id {} exceeds GDSII", layer_id.0)))?;
    report.warnings.push(GdsExportWarning {
        kind: GdsExportWarningKind::MissingLayerFallback,
        cell_id,
        shape_id: Some(shape.id),
        instance_id: None,
        layer_id: Some(layer_id),
        message: format!(
            "shape references missing layer {}; exported with that numeric GDS layer",
            layer_id.0
        ),
    });
    Ok((gds_layer, 0, 0))
}

pub(crate) fn report_ambiguous_gds_mapping_warnings(
    ambiguous_mappings: &BTreeMap<GdsExportMappingKey, Vec<LayerId>>,
    report: &mut GdsExportReport,
    cell_id: CellId,
    shape: &Shape,
    layer_id: LayerId,
    mapping: (u16, u16, u16),
) {
    let usages = match &shape.kind {
        ShapeKind::Label { .. } => vec![(true, mapping.2)],
        ShapeKind::Measurement { .. } => vec![(false, mapping.1), (true, mapping.2)],
        _ => vec![(false, mapping.1)],
    };
    for (is_text, gds_type) in usages {
        let key = GdsExportMappingKey {
            gds_layer: mapping.0,
            gds_type,
            is_text,
        };
        let Some(layer_ids) = ambiguous_mappings.get(&key) else {
            continue;
        };
        if !layer_ids.contains(&layer_id) {
            continue;
        }
        let other_layers: Vec<_> = layer_ids
            .iter()
            .copied()
            .filter(|candidate| *candidate != layer_id)
            .collect();
        if other_layers.is_empty() {
            continue;
        }
        let mapping_kind = if is_text { "text" } else { "geometry" };
        let exported_kind = if is_text { "TEXT" } else { "BOUNDARY/PATH" };
        report.warnings.push(GdsExportWarning {
            kind: GdsExportWarningKind::AmbiguousDocumentLayerMapping,
            cell_id,
            shape_id: Some(shape.id),
            instance_id: None,
            layer_id: Some(layer_id),
            message: format!(
                "layer {} shares GDSII {mapping_kind} mapping ({}, {}) with layer(s) {}; exported {exported_kind} elements will merge on that GDS pair",
                layer_id.0,
                key.gds_layer,
                key.gds_type,
                format_layer_ids(&other_layers)
            ),
        });
    }
}

pub(crate) fn format_layer_ids(layer_ids: &[LayerId]) -> String {
    layer_ids
        .iter()
        .map(|layer_id| layer_id.0.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn report_shape_metadata_warning(
    _report: &mut GdsExportReport,
    _cell_id: CellId,
    _shape: &Shape,
) {
}

pub(crate) fn report_shape_kind_warning(
    report: &mut GdsExportReport,
    cell_id: CellId,
    shape: &Shape,
    message: impl Into<String>,
) {
    report.warnings.push(GdsExportWarning {
        kind: GdsExportWarningKind::NonRoundTrippableShapeKind,
        cell_id,
        shape_id: Some(shape.id),
        instance_id: None,
        layer_id: Some(shape.layer),
        message: message.into(),
    });
}

pub(crate) fn generated_layer_color(layer: u16) -> [f32; 4] {
    let hue = (u32::from(layer) * 47) % 360;
    let phase = hue as f32 / 360.0;
    [
        0.35 + 0.45 * phase,
        0.65 - 0.25 * phase,
        0.85 - 0.35 * (phase - 0.5).abs(),
        0.44,
    ]
}

pub(crate) fn sanitize_gds_name(name: &str, fallback: &str) -> String {
    let mut sanitized = String::new();
    for character in name.chars() {
        if character.is_ascii_alphanumeric()
            || character == '_'
            || character == '$'
            || character == '?'
        {
            sanitized.push(character);
        } else if character.is_whitespace() || character == '-' {
            sanitized.push('_');
        }
    }
    if sanitized.trim_matches('_').is_empty() {
        sanitized = fallback.to_string();
    }
    if sanitized
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        sanitized.insert(0, '_');
    }
    truncate_gds_name(&sanitized)
}

pub(crate) fn truncate_gds_name(name: &str) -> String {
    name.chars().take(32).collect()
}

pub(crate) fn timestamp_fields() -> [i16; 12] {
    [2026, 1, 1, 0, 0, 0, 2026, 1, 1, 0, 0, 0]
}

pub(crate) fn divide_vector(vector: Vector, divisor: Coord) -> Vector {
    if divisor <= 0 {
        return vector;
    }
    Vector::new(vector.dx / divisor, vector.dy / divisor)
}

pub(crate) fn scale_points(
    points: &[Point],
    source_dbu_per_micron: Coord,
    target_dbu_per_micron: Coord,
    report: &mut GdsImportReport,
    context: GdsScaleWarningContext,
) -> Vec<Point> {
    points
        .iter()
        .copied()
        .map(|point| {
            scale_point(
                point,
                source_dbu_per_micron,
                target_dbu_per_micron,
                report,
                context,
            )
        })
        .collect()
}

pub(crate) fn scale_point(
    point: Point,
    source_dbu_per_micron: Coord,
    target_dbu_per_micron: Coord,
    report: &mut GdsImportReport,
    context: GdsScaleWarningContext,
) -> Point {
    Point::new(
        scale_coord_with_report(
            point.x,
            source_dbu_per_micron,
            target_dbu_per_micron,
            report,
            context,
            "x",
        ),
        scale_coord_with_report(
            point.y,
            source_dbu_per_micron,
            target_dbu_per_micron,
            report,
            context,
            "y",
        ),
    )
}

pub(crate) fn scale_vector(
    vector: Vector,
    source_dbu_per_micron: Coord,
    target_dbu_per_micron: Coord,
    report: &mut GdsImportReport,
    context: GdsScaleWarningContext,
) -> Vector {
    Vector::new(
        scale_coord_with_report(
            vector.dx,
            source_dbu_per_micron,
            target_dbu_per_micron,
            report,
            context,
            "dx",
        ),
        scale_coord_with_report(
            vector.dy,
            source_dbu_per_micron,
            target_dbu_per_micron,
            report,
            context,
            "dy",
        ),
    )
}

pub(crate) fn scale_coord_with_report(
    value: Coord,
    source_dbu_per_micron: Coord,
    target_dbu_per_micron: Coord,
    report: &mut GdsImportReport,
    context: GdsScaleWarningContext,
    component: &'static str,
) -> Coord {
    let (scaled, clamped) = scale_coord_inner(value, source_dbu_per_micron, target_dbu_per_micron);
    if let Some((rounded, clamped_value)) = clamped {
        report.warnings.push(GdsImportWarning {
            kind: GdsImportWarningKind::CoordinateClamped,
            gds_layer: context.gds_layer,
            gds_type: context.gds_type,
            is_text: context.is_text,
            layer_id: context.layer_id,
            message: format!(
                "GDS {} {component} coordinate {value} scaled to {rounded}; clamped to {clamped_value}",
                context.element_kind
            ),
        });
    }
    scaled
}

pub(crate) fn scale_coord_inner(
    value: Coord,
    source_dbu_per_micron: Coord,
    target_dbu_per_micron: Coord,
) -> (Coord, Option<(i128, Coord)>) {
    let source_effective = source_dbu_per_micron.max(1);
    let target_effective = target_dbu_per_micron.max(1);
    if source_effective != source_dbu_per_micron || target_effective != target_dbu_per_micron {
        warn!(
            source_dbu_per_micron,
            target_dbu_per_micron,
            source_effective,
            target_effective,
            "GDS coordinate scale used out-of-range dbu_per_micron; applying minimum 1"
        );
    }
    let source = i128::from(source_effective);
    let target = i128::from(target_effective);
    let numerator = i128::from(value) * target;
    let rounded = if numerator >= 0 {
        (numerator + source / 2) / source
    } else {
        (numerator - source / 2) / source
    };
    let clamped = rounded.clamp(i128::from(Coord::MIN), i128::from(Coord::MAX));
    if clamped != rounded {
        warn!(
            value,
            rounded, clamped, "scaled GDS coordinate exceeded Coord range"
        );
    }
    let scaled = clamped as Coord;
    let clamped = (clamped != rounded).then_some((rounded, scaled));
    (scaled, clamped)
}

pub(crate) fn normalize_gds_path_width(width: Coord) -> Coord {
    if width > 0 {
        width
    } else if width == Coord::MIN {
        Coord::MAX
    } else {
        width.abs().max(1)
    }
}

pub(crate) fn coord_to_i32(value: Coord) -> Result<i32, GdsError> {
    i32::try_from(value).map_err(|_| GdsError::CoordinateOverflow(value))
}

pub(crate) fn u16_to_i16(value: u16) -> Result<i16, GdsError> {
    i16::try_from(value)
        .map_err(|_| GdsError::Unsupported(format!("GDSII layer/type {value} exceeds i16")))
}

#[derive(Clone, Debug)]
pub(crate) struct Record {
    pub(crate) record_type: u8,
    pub(crate) data_type: u8,
    pub(crate) data: Vec<u8>,
}

impl Record {
    pub(crate) fn as_ascii(&self) -> Result<String, GdsError> {
        if self.data_type != ASCII {
            return Err(GdsError::InvalidRecord(format!(
                "record {:02x} is not ASCII",
                self.record_type
            )));
        }
        let text = self
            .data
            .iter()
            .copied()
            .take_while(|byte| *byte != 0)
            .collect::<Vec<_>>();
        Ok(String::from_utf8_lossy(&text).trim_end().to_string())
    }

    pub(crate) fn as_u16s(&self) -> Result<Vec<u16>, GdsError> {
        if self.data_type != INT_2 || self.data.len() % 2 != 0 {
            return Err(GdsError::InvalidRecord(format!(
                "record {:02x} is not INT_2",
                self.record_type
            )));
        }
        Ok(self
            .data
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect())
    }

    pub(crate) fn first_u16(&self) -> Result<u16, GdsError> {
        self.as_u16s()?
            .first()
            .copied()
            .ok_or(GdsError::MissingField("INT_2 value"))
    }

    pub(crate) fn first_bit_array(&self) -> Result<u16, GdsError> {
        if self.data_type != BIT_ARRAY || self.data.len() < 2 {
            return Err(GdsError::InvalidRecord(format!(
                "record {:02x} is not BIT_ARRAY",
                self.record_type
            )));
        }
        Ok(u16::from_be_bytes([self.data[0], self.data[1]]))
    }

    pub(crate) fn first_i32(&self) -> Result<i32, GdsError> {
        if self.data_type != INT_4 || self.data.len() < 4 {
            return Err(GdsError::InvalidRecord(format!(
                "record {:02x} is not INT_4",
                self.record_type
            )));
        }
        Ok(i32::from_be_bytes([
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
        ]))
    }

    pub(crate) fn as_xy(&self) -> Result<Vec<Point>, GdsError> {
        if self.data_type != INT_4 || self.data.len() % 8 != 0 {
            return Err(GdsError::InvalidRecord(
                "XY record has bad size".to_string(),
            ));
        }
        let mut points = Vec::new();
        for chunk in self.data.chunks_exact(8) {
            let x = i32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            let y = i32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
            points.push(Point::new(Coord::from(x), Coord::from(y)));
        }
        Ok(points)
    }

    pub(crate) fn as_real8(&self) -> Result<Vec<f64>, GdsError> {
        if self.data_type != REAL_8 || self.data.len() % 8 != 0 {
            return Err(GdsError::InvalidRecord(format!(
                "record {:02x} is not REAL_8",
                self.record_type
            )));
        }
        Ok(self
            .data
            .chunks_exact(8)
            .map(|chunk| {
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(chunk);
                decode_gds_real8(bytes)
            })
            .collect())
    }

    pub(crate) fn first_real8(&self) -> Result<f64, GdsError> {
        self.as_real8()?
            .first()
            .copied()
            .ok_or(GdsError::MissingField("REAL_8 value"))
    }
}

pub(crate) struct GdsReader<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) offset: usize,
}

impl<'a> GdsReader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub(crate) fn next_record(&mut self) -> Result<Option<Record>, GdsError> {
        if self.offset >= self.bytes.len() {
            return Ok(None);
        }
        if self.bytes.len() - self.offset < 4 {
            return Err(GdsError::UnexpectedEof);
        }
        let length = u16::from_be_bytes([self.bytes[self.offset], self.bytes[self.offset + 1]]);
        let length = usize::from(length);
        if length < 4 || self.offset + length > self.bytes.len() {
            return Err(GdsError::InvalidRecord(format!(
                "bad record length {length} at byte {}",
                self.offset
            )));
        }
        let record_type = self.bytes[self.offset + 2];
        let data_type = self.bytes[self.offset + 3];
        let data = self.bytes[self.offset + 4..self.offset + length].to_vec();
        self.offset += length;
        Ok(Some(Record {
            record_type,
            data_type,
            data,
        }))
    }
}

pub(crate) struct GdsWriter {
    pub(crate) bytes: Vec<u8>,
}

impl GdsWriter {
    pub(crate) fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub(crate) fn write_no_data(&mut self, record_type: u8) -> Result<(), GdsError> {
        self.write_record(record_type, NO_DATA, &[])
    }

    pub(crate) fn write_i16(&mut self, record_type: u8, values: &[i16]) -> Result<(), GdsError> {
        let mut data = Vec::with_capacity(values.len() * 2);
        for value in values {
            data.extend_from_slice(&value.to_be_bytes());
        }
        self.write_record(record_type, INT_2, &data)
    }

    pub(crate) fn write_bit_array(&mut self, record_type: u8, value: u16) -> Result<(), GdsError> {
        self.write_record(record_type, BIT_ARRAY, &value.to_be_bytes())
    }

    pub(crate) fn write_i32(&mut self, record_type: u8, values: &[i32]) -> Result<(), GdsError> {
        let mut data = Vec::with_capacity(values.len() * 4);
        for value in values {
            data.extend_from_slice(&value.to_be_bytes());
        }
        self.write_record(record_type, INT_4, &data)
    }

    pub(crate) fn write_real8(&mut self, record_type: u8, values: &[f64]) -> Result<(), GdsError> {
        let mut data = Vec::with_capacity(values.len() * 8);
        for value in values {
            data.extend_from_slice(&encode_gds_real8(*value));
        }
        self.write_record(record_type, REAL_8, &data)
    }

    pub(crate) fn write_ascii(&mut self, record_type: u8, value: &str) -> Result<(), GdsError> {
        let mut data = value.as_bytes().to_vec();
        if data.len() % 2 != 0 {
            data.push(0);
        }
        self.write_record(record_type, ASCII, &data)
    }

    pub(crate) fn write_xy(&mut self, points: &[Point]) -> Result<(), GdsError> {
        let mut data = Vec::with_capacity(points.len() * 8);
        for point in points {
            data.extend_from_slice(&coord_to_i32(point.x)?.to_be_bytes());
            data.extend_from_slice(&coord_to_i32(point.y)?.to_be_bytes());
        }
        self.write_record(XY, INT_4, &data)
    }

    pub(crate) fn write_record(
        &mut self,
        record_type: u8,
        data_type: u8,
        data: &[u8],
    ) -> Result<(), GdsError> {
        let length = data.len() + 4;
        let length = u16::try_from(length).map_err(|_| {
            GdsError::Unsupported(format!("record {record_type:02x} exceeds 65535 bytes"))
        })?;
        self.bytes.extend_from_slice(&length.to_be_bytes());
        self.bytes.push(record_type);
        self.bytes.push(data_type);
        self.bytes.extend_from_slice(data);
        Ok(())
    }
}

pub(crate) fn encode_gds_real8(value: f64) -> [u8; 8] {
    if value == 0.0 || !value.is_finite() {
        return [0; 8];
    }
    let sign = if value.is_sign_negative() { 0x80 } else { 0x00 };
    let mut mantissa = value.abs();
    let mut exponent = 0i32;
    while mantissa >= 1.0 {
        mantissa /= 16.0;
        exponent += 1;
    }
    while mantissa < 0.0625 {
        mantissa *= 16.0;
        exponent -= 1;
    }
    let mut mantissa_int = (mantissa * ((1u64 << 56) as f64)).round() as u64;
    if mantissa_int >= (1u64 << 56) {
        mantissa_int >>= 4;
        exponent += 1;
    }
    let mut bytes = [0u8; 8];
    bytes[0] = sign | ((exponent + 64) as u8 & 0x7f);
    let mantissa_bytes = mantissa_int.to_be_bytes();
    bytes[1..].copy_from_slice(&mantissa_bytes[1..]);
    bytes
}

pub(crate) fn decode_gds_real8(bytes: [u8; 8]) -> f64 {
    if bytes == [0; 8] {
        return 0.0;
    }
    let sign = if bytes[0] & 0x80 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from(bytes[0] & 0x7f) - 64;
    let mut mantissa_bytes = [0u8; 8];
    mantissa_bytes[1..].copy_from_slice(&bytes[1..]);
    let mantissa = u64::from_be_bytes(mantissa_bytes) as f64 / ((1u64 << 56) as f64);
    sign * mantissa * 16f64.powi(exponent)
}

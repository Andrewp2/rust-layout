use layout_model::{
    genealogy::LotGenealogy, inventory::FabObjectLink, workspace::WorkspaceDataset,
};

pub(crate) fn display_mask_identifier(value: &str) -> String {
    let Some(suffix) = value.strip_prefix("GLASSWORKS-DEMO-") else {
        return value.to_string();
    };
    capitalize_ascii_first(humanize_identifier(suffix))
}

pub(crate) fn display_reticle_identifier(value: &str) -> String {
    display_mask_identifier(&value.replace("-RETICLE-", "-"))
}

pub(crate) fn display_layout_revision_identifier(value: &str) -> String {
    let value = value.strip_prefix("layout_model:").unwrap_or(value);
    let value = value
        .strip_prefix("demo-")
        .or_else(|| value.strip_prefix("demo_"))
        .unwrap_or(value);
    let value = value.replace('@', " ");
    capitalize_ascii_first(humanize_identifier(&value))
}

pub(crate) fn display_route_identifier(value: &str) -> String {
    let value = value
        .strip_prefix("ROUTE-DEMO-")
        .or_else(|| value.strip_prefix("ROUTE-"))
        .unwrap_or(value);
    capitalize_ascii_first(humanize_identifier(value))
}

pub(crate) fn display_reticle_field_identifier(value: &str) -> String {
    capitalize_ascii_first(humanize_identifier(
        value.strip_prefix("F-").unwrap_or(value),
    ))
}

pub(crate) fn display_recipe_identifier(value: &str) -> String {
    value
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .enumerate()
        .map(|(index, part)| format_recipe_token(part, index == 0))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn display_measurement_identifier(value: &str) -> String {
    capitalize_ascii_first(humanize_identifier(value))
}

pub(crate) fn display_spc_control_identifier(value: &str) -> String {
    display_measurement_identifier(value.strip_suffix("_thickness").unwrap_or(value))
}

pub(crate) fn display_step_identifier(value: &str) -> String {
    capitalize_ascii_first(humanize_identifier(value))
}

pub(crate) fn display_tool_run_identifier(value: &str) -> String {
    if let Some(suffix) = value.strip_prefix("RUN-") {
        let parts = suffix
            .split('-')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if let Some((run_number, run_kind)) = parts.split_last() {
            let kind = run_kind
                .iter()
                .map(|part| part.to_ascii_lowercase())
                .collect::<Vec<_>>()
                .join(" ");
            if kind.is_empty() {
                return format!("run {run_number}");
            }
            return format!("{kind} run {run_number}");
        }
    }
    if let Some((tool_id, run_id)) = value.split_once("-RUN-") {
        return format!(
            "{} run {}",
            capitalize_ascii_first(humanize_identifier(tool_id)),
            humanize_identifier(run_id)
        );
    }
    capitalize_ascii_first(humanize_identifier(value))
}

pub(crate) fn display_lot_identifier(value: &str) -> String {
    if let Some(suffix) = value.strip_prefix("L-") {
        format!("Lot {suffix}")
    } else {
        capitalize_ascii_first(humanize_identifier(value))
    }
}

pub(crate) fn display_wafer_identifier(value: &str) -> String {
    if let Some((_, suffix)) = value.rsplit_once("-W") {
        return format!("Wafer W{suffix}");
    }
    if let Some(suffix) = value.strip_prefix('W') {
        return format!("Wafer W{suffix}");
    }
    format!(
        "Wafer {}",
        capitalize_ascii_first(humanize_identifier(value))
    )
}

pub(crate) fn display_owner_identifier(value: &str) -> String {
    let expanded = value
        .split(|ch: char| ch == '-' || ch == '_' || ch == '.' || ch.is_whitespace())
        .filter(|part| !part.is_empty())
        .map(|part| match part.to_ascii_lowercase().as_str() {
            "eng" => "engineer".to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ");
    capitalize_ascii_first(expanded)
}

pub(crate) fn display_technology_name(value: &str) -> String {
    let without_product = value
        .trim()
        .strip_prefix("Glassworks ")
        .unwrap_or(value.trim());
    capitalize_ascii_first(
        without_product
            .replace("demo", "sample")
            .replace("Demo", "Sample"),
    )
}

pub(crate) fn display_inventory_actor_identifier(value: &str) -> String {
    let words = value
        .split(|ch: char| ch == '-' || ch == '_' || ch == '.' || ch.is_whitespace())
        .filter(|part| !part.is_empty())
        .filter_map(|part| match part.to_ascii_lowercase().as_str() {
            "demo" => None,
            "op" | "ops" => Some("operator".to_string()),
            other => Some(other.to_string()),
        })
        .collect::<Vec<_>>();
    if words.is_empty() {
        "Operator".to_string()
    } else {
        capitalize_ascii_first(words.join(" "))
    }
}

pub(crate) fn compact_button_label(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let prefix_len = max_chars.saturating_sub(2);
    let prefix = value.chars().take(prefix_len).collect::<String>();
    format!("{prefix}..")
}

pub(crate) fn capitalize_ascii_first(mut value: String) -> String {
    if let Some(first_byte) = value.get_mut(0..1) {
        first_byte.make_ascii_uppercase();
    }
    value
}

fn format_recipe_token(value: &str, first: bool) -> String {
    let has_digit = value.chars().any(|ch| ch.is_ascii_digit());
    let all_upper = value.chars().all(|ch| !ch.is_ascii_lowercase());
    if value.chars().all(|ch| ch.is_ascii_digit())
        || (all_upper && value.chars().count() <= 3)
        || (has_digit && all_upper)
    {
        return value.to_string();
    }

    let mut lower = value.to_ascii_lowercase();
    if first {
        if let Some(first_byte) = lower.get_mut(0..1) {
            first_byte.make_ascii_uppercase();
        }
    }
    lower
}

pub(crate) fn humanize_identifier(value: &str) -> String {
    value
        .split(|ch: char| ch == '-' || ch == '_' || ch == '.' || ch.is_whitespace())
        .filter(|part| !part.is_empty())
        .map(|part| {
            if part.chars().count() == 1 {
                part.to_ascii_uppercase()
            } else {
                part.to_ascii_lowercase()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn workflow_lot_label<T: ToString>(
    _workspace: &WorkspaceDataset,
    lot_id: T,
    max_chars: usize,
) -> String {
    compact_button_label(&lot_id.to_string(), max_chars)
}

pub(crate) fn trace_lot_label<T: ToString>(
    _genealogy: &LotGenealogy,
    lot_id: T,
    max_chars: usize,
) -> String {
    compact_button_label(&lot_id.to_string(), max_chars)
}

pub(crate) fn trace_lot_button_label<T: ToString>(
    genealogy: &LotGenealogy,
    lot_id: T,
    compact: bool,
) -> String {
    trace_lot_label(genealogy, lot_id, if compact { 11 } else { 18 })
}

pub(crate) fn yield_wafer_short_label<T: ToString>(wafer_id: T) -> String {
    compact_button_label(&wafer_id.to_string(), 12)
}

pub(crate) fn yield_wafer_label<T: ToString>(wafer_id: T) -> String {
    wafer_id.to_string()
}

pub(crate) fn inventory_usage_link_label(value: &FabObjectLink) -> String {
    compact_button_label(&value.label(), 18)
}

pub(crate) fn display_product_label<T: ToString>(value: T) -> String {
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_helpers_remove_demo_prefixes() {
        assert_eq!(
            display_mask_identifier("GLASSWORKS-DEMO-POLY-M1"),
            "Poly m1"
        );
        assert_eq!(display_route_identifier("ROUTE-DEMO-FLOW"), "Flow");
        assert_eq!(
            display_layout_revision_identifier("layout_model:demo-poly@r3"),
            "Poly r3"
        );
        assert_eq!(
            display_technology_name("Glassworks demo process"),
            "Sample process"
        );
    }

    #[test]
    fn compact_labels_preserve_short_values_and_truncate_long_values() {
        assert_eq!(compact_button_label("Short", 8), "Short");
        assert_eq!(compact_button_label("Very long label", 8), "Very l..");
    }

    #[test]
    fn identifiers_keep_operational_tokens_readable() {
        assert_eq!(display_recipe_identifier("poly_L4_CD"), "Poly L4 CD");
        assert_eq!(display_lot_identifier("L-00042"), "Lot 00042");
        assert_eq!(display_wafer_identifier("L-00042-W07"), "Wafer W07");
        assert_eq!(
            yield_wafer_short_label("LOT-00042-WAFER-07"),
            "LOT-00042-.."
        );
    }
}

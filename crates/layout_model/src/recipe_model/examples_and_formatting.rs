#![allow(unused_imports)]
use super::*;

pub(crate) fn sample_spin_coater_recipe() -> Recipe {
    let id = RecipeId::from("SPIN_PR_3000");
    Recipe {
        id: id.clone(),
        name: "Photoresist spin coat".to_string(),
        tool_class: ToolClass::SpinCoater,
        owner: "Lithography".to_string(),
        description: "Positive resist coat for poly gate patterning.".to_string(),
        parameter_specs: specs([
            choice_spec(
                "resist",
                "Resist",
                10,
                ["AZ1512", "S1813", "PMMA A4"],
                "AZ1512",
            ),
            decimal_spec(
                "dispense_volume_ml",
                "Dispense volume",
                20,
                RecipeUnit::Milliliter,
                0.2,
                5.0,
                1.2,
            ),
            integer_spec(
                "spin_rpm",
                "Spin speed",
                30,
                RecipeUnit::Rpm,
                500,
                7000,
                4000,
            ),
            integer_spec(
                "acceleration_rpm_s",
                "Acceleration",
                40,
                RecipeUnit::RpmPerSecond,
                100,
                5000,
                1000,
            ),
            decimal_spec(
                "spin_time_s",
                "Spin time",
                50,
                RecipeUnit::Second,
                5.0,
                120.0,
                45.0,
            ),
        ]),
        versions: vec![
            RecipeVersion {
                recipe_id: id.clone(),
                version: RecipeVersionNumber(1),
                name: "Baseline 1.3 um coat".to_string(),
                change_summary: "Initial coat target for contact aligner lithography.".to_string(),
                author: "process.eng".to_string(),
                created_at: "2026-04-01T09:00:00Z".to_string(),
                approval_state: ApprovalState::Approved,
                approved_by: Some("lead.process.eng".to_string()),
                approved_at: Some("2026-04-02T15:30:00Z".to_string()),
                parameters: parameters([
                    ("resist", RecipeParameterValue::Choice("AZ1512".to_string())),
                    ("dispense_volume_ml", RecipeParameterValue::Decimal(1.2)),
                    ("spin_rpm", RecipeParameterValue::Integer(4000)),
                    ("acceleration_rpm_s", RecipeParameterValue::Integer(1000)),
                    ("spin_time_s", RecipeParameterValue::Decimal(45.0)),
                ]),
                dependencies: Vec::new(),
                approval_history: vec![approved_event()],
            },
            RecipeVersion {
                recipe_id: id.clone(),
                version: RecipeVersionNumber(2),
                name: "Reduced edge bead trial".to_string(),
                change_summary: "Slightly higher spin speed and shorter spin time.".to_string(),
                author: "litho.dev".to_string(),
                created_at: "2026-04-18T10:15:00Z".to_string(),
                approval_state: ApprovalState::InReview,
                approved_by: None,
                approved_at: None,
                parameters: parameters([
                    ("resist", RecipeParameterValue::Choice("AZ1512".to_string())),
                    ("dispense_volume_ml", RecipeParameterValue::Decimal(1.2)),
                    ("spin_rpm", RecipeParameterValue::Integer(4200)),
                    ("acceleration_rpm_s", RecipeParameterValue::Integer(1200)),
                    ("spin_time_s", RecipeParameterValue::Decimal(40.0)),
                ]),
                dependencies: Vec::new(),
                approval_history: vec![ApprovalEvent {
                    action: ApprovalAction::SubmitForReview,
                    from_state: ApprovalState::Draft,
                    to_state: ApprovalState::InReview,
                    actor: "litho.dev".to_string(),
                    timestamp: "2026-04-18T11:00:00Z".to_string(),
                    note: "Edge bead reduction trial.".to_string(),
                }],
            },
        ],
        usage_references: vec![
            route_usage(
                "SPIN_PR_3000",
                1,
                "coat_photoresist",
                "Process traveler coat step",
            ),
            tool_run_usage("SPIN_PR_3000", 1, "RUN-SPIN-0007", "SPIN-01"),
        ],
    }
}

pub(crate) fn sample_plasma_etcher_recipe() -> Recipe {
    let id = RecipeId::from("ETCH_CF4_POLY_001");
    Recipe {
        id: id.clone(),
        name: "CF4 poly etch".to_string(),
        tool_class: ToolClass::PlasmaEtcher,
        owner: "Etch".to_string(),
        description: "Timed poly etch for the demo CMOS stack.".to_string(),
        parameter_specs: specs([
            choice_spec(
                "gas_recipe",
                "Gas recipe",
                10,
                ["CF4", "CHF3/O2", "SF6"],
                "CF4",
            ),
            integer_spec(
                "pressure_mtorr",
                "Chamber pressure",
                20,
                RecipeUnit::MilliTorr,
                20,
                250,
                80,
            ),
            integer_spec("rf_power_w", "RF power", 30, RecipeUnit::Watt, 25, 500, 150),
            decimal_spec(
                "etch_time_s",
                "Etch time",
                40,
                RecipeUnit::Second,
                5.0,
                300.0,
                60.0,
            ),
            integer_spec("o2_flow_sccm", "O2 flow", 50, RecipeUnit::Sccm, 0, 100, 8),
            boolean_spec("endpoint_enabled", "Endpoint enabled", 60, false),
        ]),
        versions: vec![
            RecipeVersion {
                recipe_id: id.clone(),
                version: RecipeVersionNumber(1),
                name: "Baseline poly clear".to_string(),
                change_summary: "Qualified timed etch.".to_string(),
                author: "etch.eng".to_string(),
                created_at: "2026-03-22T14:00:00Z".to_string(),
                approval_state: ApprovalState::Approved,
                approved_by: Some("etch.lead".to_string()),
                approved_at: Some("2026-03-25T18:00:00Z".to_string()),
                parameters: parameters([
                    (
                        "gas_recipe",
                        RecipeParameterValue::Choice("CF4".to_string()),
                    ),
                    ("pressure_mtorr", RecipeParameterValue::Integer(80)),
                    ("rf_power_w", RecipeParameterValue::Integer(150)),
                    ("etch_time_s", RecipeParameterValue::Decimal(60.0)),
                    ("o2_flow_sccm", RecipeParameterValue::Integer(8)),
                    ("endpoint_enabled", RecipeParameterValue::Boolean(false)),
                ]),
                dependencies: vec![RecipeDependency {
                    recipe_id: RecipeId::from("LITHO_POLY_EXPOSE_001"),
                    version: Some(RecipeVersionNumber(1)),
                    reason: "Etch assumes the poly lithography stack from exposure v1.".to_string(),
                }],
                approval_history: vec![approved_event()],
            },
            RecipeVersion {
                recipe_id: id.clone(),
                version: RecipeVersionNumber(2),
                name: "Lower pressure split".to_string(),
                change_summary: "Lower pressure split for sidewall profile evaluation.".to_string(),
                author: "etch.dev".to_string(),
                created_at: "2026-04-20T16:45:00Z".to_string(),
                approval_state: ApprovalState::Draft,
                approved_by: None,
                approved_at: None,
                parameters: parameters([
                    (
                        "gas_recipe",
                        RecipeParameterValue::Choice("CF4".to_string()),
                    ),
                    ("pressure_mtorr", RecipeParameterValue::Integer(65)),
                    ("rf_power_w", RecipeParameterValue::Integer(150)),
                    ("etch_time_s", RecipeParameterValue::Decimal(55.0)),
                    ("o2_flow_sccm", RecipeParameterValue::Integer(8)),
                    ("endpoint_enabled", RecipeParameterValue::Boolean(false)),
                ]),
                dependencies: vec![RecipeDependency {
                    recipe_id: RecipeId::from("LITHO_POLY_EXPOSE_001"),
                    version: Some(RecipeVersionNumber(1)),
                    reason: "Same lithography dependency as baseline.".to_string(),
                }],
                approval_history: Vec::new(),
            },
        ],
        usage_references: vec![
            route_usage(
                "ETCH_CF4_POLY_001",
                1,
                "etch_poly",
                "Process traveler etch step",
            ),
            tool_run_usage("ETCH_CF4_POLY_001", 1, "RUN-ETCH-0012", "ETCH-02"),
        ],
    }
}

pub(crate) fn sample_thermal_recipe() -> Recipe {
    let id = RecipeId::from("THERMAL_BAKE_110C_001");
    Recipe {
        id: id.clone(),
        name: "Photoresist soft bake".to_string(),
        tool_class: ToolClass::FurnaceHotplate,
        owner: "Lithography".to_string(),
        description: "Hotplate or furnace bake after spin coat.".to_string(),
        parameter_specs: specs([
            choice_spec(
                "thermal_mode",
                "Thermal mode",
                10,
                ["hotplate_contact", "proximity_hotplate", "tube_furnace"],
                "hotplate_contact",
            ),
            decimal_spec(
                "temperature_c",
                "Temperature",
                20,
                RecipeUnit::Celsius,
                50.0,
                1100.0,
                110.0,
            ),
            decimal_spec(
                "duration_min",
                "Duration",
                30,
                RecipeUnit::Minute,
                0.5,
                180.0,
                1.5,
            ),
            decimal_spec(
                "ramp_rate_c_min",
                "Ramp rate",
                40,
                RecipeUnit::CelsiusPerMinute,
                0.1,
                50.0,
                10.0,
            ),
            choice_spec(
                "atmosphere",
                "Atmosphere",
                50,
                ["air", "nitrogen", "forming_gas"],
                "air",
            ),
        ]),
        versions: vec![
            RecipeVersion {
                recipe_id: id.clone(),
                version: RecipeVersionNumber(1),
                name: "Baseline hotplate bake".to_string(),
                change_summary: "Qualified post-coat soft bake.".to_string(),
                author: "litho.eng".to_string(),
                created_at: "2026-04-01T09:20:00Z".to_string(),
                approval_state: ApprovalState::Approved,
                approved_by: Some("lead.process.eng".to_string()),
                approved_at: Some("2026-04-02T15:35:00Z".to_string()),
                parameters: parameters([
                    (
                        "thermal_mode",
                        RecipeParameterValue::Choice("hotplate_contact".to_string()),
                    ),
                    ("temperature_c", RecipeParameterValue::Decimal(110.0)),
                    ("duration_min", RecipeParameterValue::Decimal(1.5)),
                    ("ramp_rate_c_min", RecipeParameterValue::Decimal(10.0)),
                    (
                        "atmosphere",
                        RecipeParameterValue::Choice("air".to_string()),
                    ),
                ]),
                dependencies: vec![RecipeDependency {
                    recipe_id: RecipeId::from("SPIN_PR_3000"),
                    version: Some(RecipeVersionNumber(1)),
                    reason: "Bake follows the qualified photoresist spin coat.".to_string(),
                }],
                approval_history: vec![approved_event()],
            },
            RecipeVersion {
                recipe_id: id.clone(),
                version: RecipeVersionNumber(2),
                name: "Nitrogen split".to_string(),
                change_summary: "Evaluate nitrogen bake atmosphere for adhesion.".to_string(),
                author: "litho.dev".to_string(),
                created_at: "2026-04-19T08:40:00Z".to_string(),
                approval_state: ApprovalState::Draft,
                approved_by: None,
                approved_at: None,
                parameters: parameters([
                    (
                        "thermal_mode",
                        RecipeParameterValue::Choice("proximity_hotplate".to_string()),
                    ),
                    ("temperature_c", RecipeParameterValue::Decimal(112.0)),
                    ("duration_min", RecipeParameterValue::Decimal(1.5)),
                    ("ramp_rate_c_min", RecipeParameterValue::Decimal(10.0)),
                    (
                        "atmosphere",
                        RecipeParameterValue::Choice("nitrogen".to_string()),
                    ),
                ]),
                dependencies: vec![RecipeDependency {
                    recipe_id: RecipeId::from("SPIN_PR_3000"),
                    version: Some(RecipeVersionNumber(1)),
                    reason: "Bake follows the qualified photoresist spin coat.".to_string(),
                }],
                approval_history: Vec::new(),
            },
        ],
        usage_references: vec![route_usage(
            "THERMAL_BAKE_110C_001",
            1,
            "soft_bake",
            "Process traveler soft bake step",
        )],
    }
}

pub(crate) fn sample_develop_recipe() -> Recipe {
    let id = RecipeId::from("LITHO_DEVELOP_001");
    Recipe {
        id: id.clone(),
        name: "Poly resist develop".to_string(),
        tool_class: ToolClass::SpinCoater,
        owner: "Lithography".to_string(),
        description: "Developer puddle and rinse after poly exposure.".to_string(),
        parameter_specs: specs([
            choice_spec(
                "developer",
                "Developer",
                10,
                ["AZ 300 MIF", "MF-319", "TMAH 2.38%"],
                "AZ 300 MIF",
            ),
            decimal_spec(
                "develop_time_s",
                "Develop time",
                20,
                RecipeUnit::Second,
                10.0,
                180.0,
                60.0,
            ),
            decimal_spec(
                "rinse_time_s",
                "Rinse time",
                30,
                RecipeUnit::Second,
                5.0,
                120.0,
                30.0,
            ),
            integer_spec("dry_rpm", "Dry spin", 40, RecipeUnit::Rpm, 500, 5000, 2500),
        ]),
        versions: vec![RecipeVersion {
            recipe_id: id.clone(),
            version: RecipeVersionNumber(1),
            name: "Baseline develop".to_string(),
            change_summary: "Qualified puddle develop for the demo poly mask.".to_string(),
            author: "litho.process".to_string(),
            created_at: "2026-04-03T10:00:00Z".to_string(),
            approval_state: ApprovalState::Approved,
            approved_by: Some("lead.process.eng".to_string()),
            approved_at: Some("2026-04-04T15:30:00Z".to_string()),
            parameters: parameters([
                (
                    "developer",
                    RecipeParameterValue::Choice("AZ 300 MIF".to_string()),
                ),
                ("develop_time_s", RecipeParameterValue::Decimal(60.0)),
                ("rinse_time_s", RecipeParameterValue::Decimal(30.0)),
                ("dry_rpm", RecipeParameterValue::Integer(2500)),
            ]),
            dependencies: vec![RecipeDependency {
                recipe_id: RecipeId::from("LITHO_POLY_EXPOSE_001"),
                version: Some(RecipeVersionNumber(1)),
                reason: "Develop is qualified against the poly exposure dose.".to_string(),
            }],
            approval_history: vec![approved_event()],
        }],
        usage_references: vec![route_usage(
            "LITHO_DEVELOP_001",
            1,
            "develop_resist",
            "Process traveler develop step",
        )],
    }
}

pub(crate) fn sample_lithography_recipe() -> Recipe {
    let id = RecipeId::from("LITHO_POLY_EXPOSE_001");
    Recipe {
        id: id.clone(),
        name: "Poly lithography exposure".to_string(),
        tool_class: ToolClass::LithographyExposure,
        owner: "Lithography".to_string(),
        description: "Mask aligner exposure recipe for poly gate definition.".to_string(),
        parameter_specs: specs([
            text_spec("mask_id", "Mask ID", 10, "RETICLE-POLY-DEMO"),
            decimal_spec(
                "exposure_dose_mj_cm2",
                "Exposure dose",
                20,
                RecipeUnit::MillijoulePerSquareCentimeter,
                20.0,
                300.0,
                95.0,
            ),
            decimal_spec(
                "focus_offset_um",
                "Focus offset",
                30,
                RecipeUnit::Micrometer,
                -10.0,
                10.0,
                0.0,
            ),
            integer_spec(
                "wavelength_nm",
                "Wavelength",
                40,
                RecipeUnit::Nanometer,
                250,
                450,
                365,
            ),
            choice_spec(
                "alignment_mode",
                "Alignment mode",
                50,
                ["global", "local", "manual"],
                "global",
            ),
            decimal_spec(
                "contact_force_percent",
                "Contact force",
                60,
                RecipeUnit::Percent,
                0.0,
                100.0,
                35.0,
            ),
        ]),
        versions: vec![
            RecipeVersion {
                recipe_id: id.clone(),
                version: RecipeVersionNumber(1),
                name: "Baseline poly expose".to_string(),
                change_summary: "Qualified exposure dose for AZ1512 on demo poly.".to_string(),
                author: "litho.eng".to_string(),
                created_at: "2026-04-01T10:00:00Z".to_string(),
                approval_state: ApprovalState::Approved,
                approved_by: Some("lead.process.eng".to_string()),
                approved_at: Some("2026-04-03T13:15:00Z".to_string()),
                parameters: parameters([
                    (
                        "mask_id",
                        RecipeParameterValue::Text("RETICLE-POLY-DEMO".to_string()),
                    ),
                    ("exposure_dose_mj_cm2", RecipeParameterValue::Decimal(95.0)),
                    ("focus_offset_um", RecipeParameterValue::Decimal(0.0)),
                    ("wavelength_nm", RecipeParameterValue::Integer(365)),
                    (
                        "alignment_mode",
                        RecipeParameterValue::Choice("global".to_string()),
                    ),
                    ("contact_force_percent", RecipeParameterValue::Decimal(35.0)),
                ]),
                dependencies: vec![RecipeDependency {
                    recipe_id: RecipeId::from("THERMAL_BAKE_110C_001"),
                    version: Some(RecipeVersionNumber(1)),
                    reason: "Exposure assumes the qualified soft bake.".to_string(),
                }],
                approval_history: vec![approved_event()],
            },
            RecipeVersion {
                recipe_id: id.clone(),
                version: RecipeVersionNumber(2),
                name: "Dose focus split".to_string(),
                change_summary: "Slight dose reduction for line-width split.".to_string(),
                author: "litho.dev".to_string(),
                created_at: "2026-04-21T12:10:00Z".to_string(),
                approval_state: ApprovalState::Draft,
                approved_by: None,
                approved_at: None,
                parameters: parameters([
                    (
                        "mask_id",
                        RecipeParameterValue::Text("RETICLE-POLY-DEMO".to_string()),
                    ),
                    ("exposure_dose_mj_cm2", RecipeParameterValue::Decimal(88.0)),
                    ("focus_offset_um", RecipeParameterValue::Decimal(0.4)),
                    ("wavelength_nm", RecipeParameterValue::Integer(365)),
                    (
                        "alignment_mode",
                        RecipeParameterValue::Choice("global".to_string()),
                    ),
                    ("contact_force_percent", RecipeParameterValue::Decimal(35.0)),
                ]),
                dependencies: vec![RecipeDependency {
                    recipe_id: RecipeId::from("THERMAL_BAKE_110C_001"),
                    version: Some(RecipeVersionNumber(1)),
                    reason: "Exposure assumes the qualified soft bake.".to_string(),
                }],
                approval_history: Vec::new(),
            },
        ],
        usage_references: vec![route_usage(
            "LITHO_POLY_EXPOSE_001",
            1,
            "expose_poly",
            "Process traveler exposure step",
        )],
    }
}

pub(crate) fn sample_cd_metrology_recipe() -> Recipe {
    let id = RecipeId::from("METRO_POLY_CD_001");
    Recipe {
        id: id.clone(),
        name: "Poly CD metrology".to_string(),
        tool_class: ToolClass::LithographyExposure,
        owner: "Metrology".to_string(),
        description: "Nine-site critical-dimension measurement after poly etch.".to_string(),
        parameter_specs: specs([
            text_spec("target_feature", "Target feature", 10, "poly gate"),
            integer_spec(
                "sites_per_wafer",
                "Sites per wafer",
                20,
                RecipeUnit::Count,
                1,
                49,
                9,
            ),
            decimal_spec(
                "target_cd_nm",
                "Target CD",
                30,
                RecipeUnit::Nanometer,
                10.0,
                500.0,
                45.0,
            ),
        ]),
        versions: vec![RecipeVersion {
            recipe_id: id.clone(),
            version: RecipeVersionNumber(1),
            name: "Baseline poly CD map".to_string(),
            change_summary: "Initial nine-site CD sampling plan for the demo poly module."
                .to_string(),
            author: "metrology.process".to_string(),
            created_at: "2026-04-05T09:45:00Z".to_string(),
            approval_state: ApprovalState::Approved,
            approved_by: Some("lead.process.eng".to_string()),
            approved_at: Some("2026-04-05T16:00:00Z".to_string()),
            parameters: parameters([
                (
                    "target_feature",
                    RecipeParameterValue::Text("poly gate".to_string()),
                ),
                ("sites_per_wafer", RecipeParameterValue::Integer(9)),
                ("target_cd_nm", RecipeParameterValue::Decimal(45.0)),
            ]),
            dependencies: vec![RecipeDependency {
                recipe_id: RecipeId::from("ETCH_CF4_POLY_001"),
                version: Some(RecipeVersionNumber(1)),
                reason: "Measurement validates the poly etch step.".to_string(),
            }],
            approval_history: vec![approved_event()],
        }],
        usage_references: vec![route_usage(
            "METRO_POLY_CD_001",
            1,
            "poly_cd_metrology",
            "Process traveler CD metrology step",
        )],
    }
}

pub(crate) fn sample_process_route() -> ProcessRoute {
    ProcessRoute {
        id: ProcessRouteId::from("ROUTE_POLY_GATE_DEMO"),
        version: 1,
        name: "Demo poly gate traveler".to_string(),
        steps: vec![
            ProcessStep {
                id: ProcessStepId::from("coat_photoresist"),
                sequence: 10,
                name: "Coat photoresist".to_string(),
                tool_class: ToolClass::SpinCoater,
                recipe: Some(RecipeBinding::new("SPIN_PR_3000", 1)),
                operator_signoff_required: true,
            },
            ProcessStep {
                id: ProcessStepId::from("soft_bake"),
                sequence: 20,
                name: "Soft bake".to_string(),
                tool_class: ToolClass::FurnaceHotplate,
                recipe: Some(RecipeBinding::new("THERMAL_BAKE_110C_001", 1)),
                operator_signoff_required: true,
            },
            ProcessStep {
                id: ProcessStepId::from("expose_poly"),
                sequence: 30,
                name: "Expose poly".to_string(),
                tool_class: ToolClass::LithographyExposure,
                recipe: Some(RecipeBinding::new("LITHO_POLY_EXPOSE_001", 1)),
                operator_signoff_required: true,
            },
            ProcessStep {
                id: ProcessStepId::from("etch_poly"),
                sequence: 40,
                name: "Etch poly".to_string(),
                tool_class: ToolClass::PlasmaEtcher,
                recipe: Some(RecipeBinding::new("ETCH_CF4_POLY_001", 1)),
                operator_signoff_required: true,
            },
        ],
    }
}

pub(crate) fn sample_spin_tool_run() -> ToolRun {
    ToolRun {
        id: ToolRunId::from("RUN-SPIN-0007"),
        tool_id: "SPIN-01".to_string(),
        tool_class: ToolClass::SpinCoater,
        recipe: RecipeBinding::new("SPIN_PR_3000", 1),
        process_step: Some(ProcessStepReference {
            route_id: ProcessRouteId::from("ROUTE_POLY_GATE_DEMO"),
            route_version: 1,
            step_id: ProcessStepId::from("coat_photoresist"),
        }),
        state: ToolRunState::Completed,
        started_at: "2026-04-22T09:15:00Z".to_string(),
        completed_at: Some("2026-04-22T09:18:00Z".to_string()),
    }
}

pub(crate) fn sample_etch_tool_run() -> ToolRun {
    ToolRun {
        id: ToolRunId::from("RUN-ETCH-0012"),
        tool_id: "ETCH-02".to_string(),
        tool_class: ToolClass::PlasmaEtcher,
        recipe: RecipeBinding::new("ETCH_CF4_POLY_001", 1),
        process_step: Some(ProcessStepReference {
            route_id: ProcessRouteId::from("ROUTE_POLY_GATE_DEMO"),
            route_version: 1,
            step_id: ProcessStepId::from("etch_poly"),
        }),
        state: ToolRunState::Completed,
        started_at: "2026-04-22T11:00:00Z".to_string(),
        completed_at: Some("2026-04-22T11:03:00Z".to_string()),
    }
}

pub(crate) fn specs(
    values: impl IntoIterator<Item = (&'static str, RecipeParameterSpec)>,
) -> BTreeMap<String, RecipeParameterSpec> {
    values
        .into_iter()
        .map(|(key, spec)| (key.to_string(), spec))
        .collect()
}

pub(crate) fn parameters(
    values: impl IntoIterator<Item = (&'static str, RecipeParameterValue)>,
) -> BTreeMap<String, RecipeParameterValue> {
    values
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

pub(crate) fn decimal_spec(
    key: &'static str,
    label: &'static str,
    display_order: u32,
    unit: RecipeUnit,
    min: f64,
    max: f64,
    default_value: f64,
) -> (&'static str, RecipeParameterSpec) {
    (
        key,
        RecipeParameterSpec {
            key: key.to_string(),
            label: label.to_string(),
            value_type: RecipeParameterType::Decimal,
            unit: Some(unit),
            required: true,
            display_order,
            description: String::new(),
            validation_rules: vec![ValidationRule::numeric_range(Some(min), Some(max))],
            default_value: Some(RecipeParameterValue::Decimal(default_value)),
        },
    )
}

pub(crate) fn integer_spec(
    key: &'static str,
    label: &'static str,
    display_order: u32,
    unit: RecipeUnit,
    min: i64,
    max: i64,
    default_value: i64,
) -> (&'static str, RecipeParameterSpec) {
    (
        key,
        RecipeParameterSpec {
            key: key.to_string(),
            label: label.to_string(),
            value_type: RecipeParameterType::Integer,
            unit: Some(unit),
            required: true,
            display_order,
            description: String::new(),
            validation_rules: vec![ValidationRule::integer_range(Some(min), Some(max))],
            default_value: Some(RecipeParameterValue::Integer(default_value)),
        },
    )
}

pub(crate) fn boolean_spec(
    key: &'static str,
    label: &'static str,
    display_order: u32,
    default_value: bool,
) -> (&'static str, RecipeParameterSpec) {
    (
        key,
        RecipeParameterSpec {
            key: key.to_string(),
            label: label.to_string(),
            value_type: RecipeParameterType::Boolean,
            unit: None,
            required: true,
            display_order,
            description: String::new(),
            validation_rules: Vec::new(),
            default_value: Some(RecipeParameterValue::Boolean(default_value)),
        },
    )
}

pub(crate) fn text_spec(
    key: &'static str,
    label: &'static str,
    display_order: u32,
    default_value: &'static str,
) -> (&'static str, RecipeParameterSpec) {
    (
        key,
        RecipeParameterSpec {
            key: key.to_string(),
            label: label.to_string(),
            value_type: RecipeParameterType::Text,
            unit: None,
            required: true,
            display_order,
            description: String::new(),
            validation_rules: vec![ValidationRule::NonEmpty],
            default_value: Some(RecipeParameterValue::Text(default_value.to_string())),
        },
    )
}

pub(crate) fn choice_spec<const N: usize>(
    key: &'static str,
    label: &'static str,
    display_order: u32,
    options: [&'static str; N],
    default_value: &'static str,
) -> (&'static str, RecipeParameterSpec) {
    let options: Vec<_> = options.into_iter().map(str::to_string).collect();
    (
        key,
        RecipeParameterSpec {
            key: key.to_string(),
            label: label.to_string(),
            value_type: RecipeParameterType::Choice {
                options: options.clone(),
            },
            unit: None,
            required: true,
            display_order,
            description: String::new(),
            validation_rules: Vec::new(),
            default_value: Some(RecipeParameterValue::Choice(default_value.to_string())),
        },
    )
}

pub(crate) fn approved_event() -> ApprovalEvent {
    ApprovalEvent {
        action: ApprovalAction::Approve,
        from_state: ApprovalState::InReview,
        to_state: ApprovalState::Approved,
        actor: "lead.process.eng".to_string(),
        timestamp: "2026-04-02T15:30:00Z".to_string(),
        note: "Initial qualified version.".to_string(),
    }
}

pub(crate) fn route_usage(
    recipe_id: &'static str,
    version: u32,
    step_id: &'static str,
    context: &'static str,
) -> RecipeUsageReference {
    RecipeUsageReference {
        binding: RecipeBinding::new(recipe_id, version),
        target: RecipeUsageTarget::ProcessStep {
            route_id: ProcessRouteId::from("ROUTE_POLY_GATE_DEMO"),
            route_version: 1,
            step_id: ProcessStepId::from(step_id),
        },
        context: context.to_string(),
    }
}

pub(crate) fn tool_run_usage(
    recipe_id: &'static str,
    version: u32,
    tool_run_id: &'static str,
    tool_id: &'static str,
) -> RecipeUsageReference {
    RecipeUsageReference {
        binding: RecipeBinding::new(recipe_id, version),
        target: RecipeUsageTarget::ToolRun {
            tool_run_id: ToolRunId::from(tool_run_id),
            tool_id: tool_id.to_string(),
        },
        context: "Historical tool run".to_string(),
    }
}

pub(crate) fn range_f64_failed(
    value: f64,
    min: Option<f64>,
    max: Option<f64>,
    inclusive: bool,
) -> bool {
    if inclusive {
        min.is_some_and(|min| value < min) || max.is_some_and(|max| value > max)
    } else {
        min.is_some_and(|min| value <= min) || max.is_some_and(|max| value >= max)
    }
}

pub(crate) fn range_i64_failed(
    value: i64,
    min: Option<i64>,
    max: Option<i64>,
    inclusive: bool,
) -> bool {
    if inclusive {
        min.is_some_and(|min| value < min) || max.is_some_and(|max| value > max)
    } else {
        min.is_some_and(|min| value <= min) || max.is_some_and(|max| value >= max)
    }
}

pub(crate) fn format_numeric_range(min: Option<f64>, max: Option<f64>, inclusive: bool) -> String {
    match (min, max, inclusive) {
        (Some(min), Some(max), true) => format!(
            "between {} and {}",
            format_decimal(min),
            format_decimal(max)
        ),
        (Some(min), Some(max), false) => format!(
            "greater than {} and less than {}",
            format_decimal(min),
            format_decimal(max)
        ),
        (Some(min), None, true) => format!("at least {}", format_decimal(min)),
        (Some(min), None, false) => format!("greater than {}", format_decimal(min)),
        (None, Some(max), true) => format!("at most {}", format_decimal(max)),
        (None, Some(max), false) => format!("less than {}", format_decimal(max)),
        (None, None, _) => "finite".to_string(),
    }
}

pub(crate) fn format_integer_range(min: Option<i64>, max: Option<i64>, inclusive: bool) -> String {
    match (min, max, inclusive) {
        (Some(min), Some(max), true) => format!("between {min} and {max}"),
        (Some(min), Some(max), false) => format!("greater than {min} and less than {max}"),
        (Some(min), None, true) => format!("at least {min}"),
        (Some(min), None, false) => format!("greater than {min}"),
        (None, Some(max), true) => format!("at most {max}"),
        (None, Some(max), false) => format!("less than {max}"),
        (None, None, _) => "finite".to_string(),
    }
}

pub(crate) fn format_decimal(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let mut rendered = format!("{value:.3}");
    while rendered.contains('.') && rendered.ends_with('0') {
        rendered.pop();
    }
    if rendered.ends_with('.') {
        rendered.pop();
    }
    rendered
}

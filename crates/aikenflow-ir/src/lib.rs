use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Protocol {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub assets: Vec<AssetDecl>,
    #[serde(default)]
    pub states: Vec<StateDecl>,
    #[serde(default)]
    pub transitions: Vec<TransitionDecl>,
    #[serde(default)]
    pub invariants: Vec<InvariantDecl>,
}

impl Protocol {
    pub fn state(&self, name: &str) -> Option<&StateDecl> {
        self.states.iter().find(|state| state.name == name)
    }

    pub fn transitions_consuming_state<'a>(
        &'a self,
        state_name: &'a str,
    ) -> impl Iterator<Item = &'a TransitionDecl> + 'a {
        self.transitions.iter().filter(move |transition| {
            transition
                .inputs
                .iter()
                .any(|input| input.state == state_name)
        })
    }

    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if self.name.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "AF001",
                "Protocol name is required.",
                "Set the top-level `protocol` field.",
            ));
        } else if !is_codegen_safe_name(&self.name) {
            diagnostics.push(Diagnostic::error(
                "AF061",
                format!(
                    "Protocol name `{}` is not safe for generated identifiers.",
                    self.name
                ),
                "Use a name that normalizes to an identifier beginning with an ASCII letter and is not a reserved word, such as `SimpleVault`.",
            ));
        }

        if self.states.is_empty() {
            diagnostics.push(Diagnostic::error(
                "AF002",
                "Protocol must declare at least one state.",
                "Add a `states` map with one or more named states.",
            ));
        }

        let mut asset_names = BTreeSet::new();
        let mut asset_kinds = BTreeMap::new();
        let mut asset_policies = BTreeMap::new();
        for asset in &self.assets {
            if asset.name.trim().is_empty() {
                diagnostics.push(Diagnostic::error(
                    "AF010",
                    "Asset name cannot be empty.",
                    "Use a stable asset name such as `ada` or `governance_token`.",
                ));
            } else if !asset_names.insert(asset.name.clone()) {
                diagnostics.push(Diagnostic::error(
                    "AF011",
                    format!("Asset `{}` is declared more than once.", asset.name),
                    "Asset names must be unique.",
                ));
            }

            if !asset.name.trim().is_empty() {
                asset_kinds.insert(asset.name.clone(), asset.kind.as_str());
                asset_policies.insert(asset.name.clone(), asset.policy.as_deref());
            }

            if asset.kind.trim().is_empty() {
                diagnostics.push(Diagnostic::error(
                    "AF012",
                    format!("Asset `{}` has an empty kind.", asset.name),
                    "Use an asset kind such as `lovelace` or `native`.",
                ));
            }
        }

        let mut state_names = BTreeSet::new();
        let mut generated_state_names = BTreeMap::<String, String>::new();
        for state in &self.states {
            if state.name.trim().is_empty() {
                diagnostics.push(Diagnostic::error(
                    "AF003",
                    "State name cannot be empty.",
                    "Use a descriptive state name such as `Locked` or `Open`.",
                ));
            } else if !state_names.insert(state.name.clone()) {
                diagnostics.push(Diagnostic::error(
                    "AF004",
                    format!("State `{}` is declared more than once.", state.name),
                    "State names must be unique.",
                ));
            } else if !is_codegen_safe_name(&state.name) {
                diagnostics.push(Diagnostic::error(
                    "AF062",
                    format!(
                        "State name `{}` is not safe for generated identifiers.",
                        state.name
                    ),
                    "Use a state name that normalizes to an identifier beginning with an ASCII letter and is not a reserved word.",
                ));
            }

            let generated_state_name = to_snake_case(&state.name);
            if !generated_state_name.is_empty() {
                if let Some(previous) =
                    generated_state_names.insert(generated_state_name.clone(), state.name.clone())
                {
                    diagnostics.push(Diagnostic::error(
                        "AF055",
                        format!(
                            "State `{}` collides with `{}` after code generation normalization.",
                            state.name, previous
                        ),
                        "Use state names that remain distinct after snake_case/PascalCase normalization.",
                    ));
                }
            }

            let mut field_names = BTreeSet::new();
            let mut generated_field_names = BTreeMap::<String, String>::new();
            for field in &state.datum {
                if field.name.trim().is_empty() {
                    diagnostics.push(Diagnostic::error(
                        "AF005",
                        format!(
                            "State `{}` has a datum field with an empty name.",
                            state.name
                        ),
                        "Use a named datum field.",
                    ));
                } else if !field_names.insert(field.name.clone()) {
                    diagnostics.push(Diagnostic::error(
                        "AF006",
                        format!(
                            "State `{}` declares datum field `{}` more than once.",
                            state.name, field.name
                        ),
                        "Datum field names must be unique within a state.",
                    ));
                } else if !is_codegen_safe_name(&field.name) {
                    diagnostics.push(Diagnostic::error(
                        "AF064",
                        format!(
                            "State `{}` datum field `{}` is not safe for generated identifiers.",
                            state.name, field.name
                        ),
                        "Use a datum field name that normalizes to an identifier beginning with an ASCII letter and is not a reserved word.",
                    ));
                }

                let generated_field_name = to_snake_case(&field.name);
                if !generated_field_name.is_empty() {
                    if let Some(previous) = generated_field_names
                        .insert(generated_field_name.clone(), field.name.clone())
                    {
                        diagnostics.push(Diagnostic::error(
                            "AF057",
                            format!(
                                "State `{}` datum field `{}` collides with `{}` after code generation normalization.",
                                state.name, field.name, previous
                            ),
                            "Use datum field names that remain distinct after snake_case/camelCase normalization.",
                        ));
                    }
                }

                if field.ty.trim().is_empty() {
                    diagnostics.push(Diagnostic::error(
                        "AF007",
                        format!(
                            "State `{}` datum field `{}` has an empty type.",
                            state.name, field.name
                        ),
                        "Set a datum field type such as `PubKeyHash`, `Lovelace`, or `Int`.",
                    ));
                }
            }
        }

        let terminal_state_names = self
            .states
            .iter()
            .filter(|state| state.terminal)
            .map(|state| state.name.clone())
            .collect::<BTreeSet<_>>();
        let state_field_names = self
            .states
            .iter()
            .map(|state| {
                (
                    state.name.clone(),
                    state
                        .datum
                        .iter()
                        .map(|field| field.name.clone())
                        .collect::<BTreeSet<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();

        if self.transitions.is_empty() {
            diagnostics.push(Diagnostic::warning(
                "AF020",
                "Protocol has no transitions.",
                "Add at least one transition so AikenFlow can generate topology and builders.",
            ));
        }

        let mut transition_names = BTreeSet::new();
        let mut generated_transition_names = BTreeMap::<String, String>::new();
        for transition in &self.transitions {
            if transition.name.trim().is_empty() {
                diagnostics.push(Diagnostic::error(
                    "AF021",
                    "Transition name cannot be empty.",
                    "Use a descriptive transition name such as `Deposit` or `Withdraw`.",
                ));
            } else if !transition_names.insert(transition.name.clone()) {
                diagnostics.push(Diagnostic::error(
                    "AF022",
                    format!(
                        "Transition `{}` is declared more than once.",
                        transition.name
                    ),
                    "Transition names must be unique.",
                ));
            } else if !is_codegen_safe_name(&transition.name) {
                diagnostics.push(Diagnostic::error(
                    "AF063",
                    format!(
                        "Transition name `{}` is not safe for generated identifiers.",
                        transition.name
                    ),
                    "Use a transition name that normalizes to an identifier beginning with an ASCII letter and is not a reserved word.",
                ));
            }

            let generated_transition_name = to_snake_case(&transition.name);
            if !generated_transition_name.is_empty() {
                if let Some(previous) = generated_transition_names
                    .insert(generated_transition_name.clone(), transition.name.clone())
                {
                    diagnostics.push(Diagnostic::error(
                        "AF056",
                        format!(
                            "Transition `{}` collides with `{}` after code generation normalization.",
                            transition.name, previous
                        ),
                        "Use transition names that remain distinct after snake_case/camelCase/PascalCase normalization.",
                    ));
                }
            }

            if transition.inputs.is_empty()
                && transition.outputs.is_empty()
                && transition.mints.is_empty()
                && transition.burns.is_empty()
            {
                diagnostics.push(Diagnostic::warning(
                    "AF023",
                    format!(
                        "Transition `{}` has no inputs, outputs, mint effects, or burn effects.",
                        transition.name
                    ),
                    "Most useful transitions consume, produce, mint, or burn something.",
                ));
            }

            for input in transition.inputs.iter().chain(&transition.reference_inputs) {
                if input.state.trim().is_empty() {
                    diagnostics.push(Diagnostic::error(
                        "AF026",
                        format!(
                            "Transition `{}` has an input with an empty state.",
                            transition.name
                        ),
                        "Set every input and reference input to a declared state name.",
                    ));
                } else if !state_names.contains(&input.state) {
                    diagnostics.push(Diagnostic::error(
                        "AF024",
                        format!(
                            "Transition `{}` references unknown input state `{}`.",
                            transition.name, input.state
                        ),
                        "Declare the state under `states` or fix the transition input.",
                    ));
                }
            }

            Self::validate_input_bindings(
                transition,
                &transition.inputs,
                "input",
                &mut diagnostics,
            );
            Self::validate_input_bindings(
                transition,
                &transition.reference_inputs,
                "reference input",
                &mut diagnostics,
            );
            Self::validate_output_bindings(transition, &mut diagnostics);

            for input in &transition.inputs {
                if terminal_state_names.contains(&input.state) {
                    diagnostics.push(Diagnostic::error(
                        "AF027",
                        format!(
                            "Transition `{}` consumes terminal state `{}`.",
                            transition.name, input.state
                        ),
                        "Terminal states may be produced but should not be consumed by later transitions.",
                    ));
                }
            }

            for output in &transition.outputs {
                if output.state.trim().is_empty() {
                    diagnostics.push(Diagnostic::error(
                        "AF028",
                        format!(
                            "Transition `{}` has an output with an empty state.",
                            transition.name
                        ),
                        "Set every output to a declared state name.",
                    ));
                } else if !state_names.contains(&output.state) {
                    diagnostics.push(Diagnostic::error(
                        "AF025",
                        format!(
                            "Transition `{}` references unknown output state `{}`.",
                            transition.name, output.state
                        ),
                        "Declare the state under `states` or fix the transition output.",
                    ));
                }

                Self::validate_output_assignments(
                    transition,
                    output,
                    &asset_names,
                    &state_field_names,
                    &mut diagnostics,
                );

                if transition.inputs.is_empty() && output.value.is_empty() {
                    diagnostics.push(Diagnostic::warning(
                        "AF050",
                        format!(
                            "Transition `{}` creates output state `{}` without value assignments.",
                            transition.name, output.state
                        ),
                        "Add an explicit `outputs[].value` map. Off-chain builders cannot infer creation output assets and will emit a runtime error for this output.",
                    ));
                }
            }

            Self::validate_asset_effects(
                transition,
                "mint",
                &transition.mints,
                &asset_names,
                &asset_kinds,
                &asset_policies,
                &mut diagnostics,
            );
            Self::validate_asset_effects(
                transition,
                "burn",
                &transition.burns,
                &asset_names,
                &asset_kinds,
                &asset_policies,
                &mut diagnostics,
            );
            Self::validate_constraint_values(transition, &mut diagnostics);
            self.validate_constraint_type_compatibility(transition, &mut diagnostics);

            for constraint in &transition.constraints {
                if let Constraint::Unknown { name, .. } = constraint {
                    diagnostics.push(Diagnostic::warning(
                        "AF030",
                        format!(
                            "Transition `{}` uses unknown constraint `{}`.",
                            transition.name, name
                        ),
                        "AikenFlow will preserve this in reports, but generators will not enforce it.",
                    ));
                }
            }

            self.warn_on_missing_owner_signature(transition, &mut diagnostics);
        }

        if self.invariants.is_empty() {
            diagnostics.push(Diagnostic::warning(
                "AF040",
                "Protocol declares no invariants.",
                "Add invariant expressions so assurance output is useful to reviewers.",
            ));
        }

        let mut invariant_names = BTreeSet::new();
        for invariant in &self.invariants {
            if invariant.name.trim().is_empty() {
                diagnostics.push(Diagnostic::error(
                    "AF041",
                    "Invariant name cannot be empty.",
                    "Use a stable invariant name such as `withdrawal_requires_owner`.",
                ));
            } else if !invariant_names.insert(invariant.name.clone()) {
                diagnostics.push(Diagnostic::error(
                    "AF042",
                    format!("Invariant `{}` is declared more than once.", invariant.name),
                    "Invariant names must be unique.",
                ));
            }

            if invariant.expression.trim().is_empty() {
                diagnostics.push(Diagnostic::error(
                    "AF043",
                    format!("Invariant `{}` has an empty expression.", invariant.name),
                    "Describe the property that should be reviewed or tested.",
                ));
            }
        }

        diagnostics
    }

    pub fn has_validation_errors(&self) -> bool {
        self.validate()
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    }

    pub fn topology(&self) -> Vec<TransitionTopology> {
        self.transitions
            .iter()
            .map(|transition| TransitionTopology {
                transition: transition.name.clone(),
                consumes: transition
                    .inputs
                    .iter()
                    .map(|input| input.state.clone())
                    .collect(),
                references: transition
                    .reference_inputs
                    .iter()
                    .map(|input| input.state.clone())
                    .collect(),
                produces: transition
                    .outputs
                    .iter()
                    .map(|output| output.state.clone())
                    .collect(),
                mints: transition
                    .mints
                    .iter()
                    .map(|effect| {
                        format!("{}::{} = {}", effect.policy, effect.asset, effect.amount)
                    })
                    .collect(),
                burns: transition
                    .burns
                    .iter()
                    .map(|effect| {
                        format!("{}::{} = {}", effect.policy, effect.asset, effect.amount)
                    })
                    .collect(),
                requires_signatures: transition
                    .constraints
                    .iter()
                    .filter_map(Constraint::signature_requirement)
                    .map(str::to_owned)
                    .collect(),
                requires_validity: transition
                    .constraints
                    .iter()
                    .filter_map(Constraint::validity_requirement)
                    .map(|(direction, point)| (direction.to_owned(), point.to_owned()))
                    .collect(),
            })
            .collect()
    }

    fn warn_on_missing_owner_signature(
        &self,
        transition: &TransitionDecl,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let has_signature_constraint = transition
            .constraints
            .iter()
            .any(|constraint| matches!(constraint, Constraint::SignedBy(_)));

        if has_signature_constraint {
            return;
        }

        for input in &transition.inputs {
            let Some(state) = self.state(&input.state) else {
                continue;
            };

            if state.datum.iter().any(|field| field.name == "owner") {
                diagnostics.push(Diagnostic::warning(
                    "AF031",
                    format!(
                        "Transition `{}` consumes `{}` but does not require a signature.",
                        transition.name, state.name
                    ),
                    "Consider adding `- signed_by: \"datum.owner\"` unless this transition is intentionally permissionless.",
                ));
            }
        }
    }

    fn validate_constraint_values(transition: &TransitionDecl, diagnostics: &mut Vec<Diagnostic>) {
        for constraint in &transition.constraints {
            match constraint {
                Constraint::SignedBy(value) if value.trim().is_empty() => {
                    diagnostics.push(Diagnostic::error(
                        "AF032",
                        format!(
                            "Transition `{}` has an empty `signed_by` constraint.",
                            transition.name
                        ),
                        "`signed_by` must name a signer expression such as `datum.owner`.",
                    ));
                }
                Constraint::Before(value) if value.trim().is_empty() => {
                    diagnostics.push(Diagnostic::error(
                        "AF033",
                        format!(
                            "Transition `{}` has an empty `before` constraint.",
                            transition.name
                        ),
                        "`before` must name a validity upper bound expression.",
                    ));
                }
                Constraint::After(value) if value.trim().is_empty() => {
                    diagnostics.push(Diagnostic::error(
                        "AF034",
                        format!(
                            "Transition `{}` has an empty `after` constraint.",
                            transition.name
                        ),
                        "`after` must name a validity lower bound expression.",
                    ));
                }
                Constraint::Positive(value) if value.trim().is_empty() => {
                    diagnostics.push(Diagnostic::error(
                        "AF035",
                        format!(
                            "Transition `{}` has an empty `positive` constraint.",
                            transition.name
                        ),
                        "`positive` must name an amount expression.",
                    ));
                }
                Constraint::DatumFieldEquals { field, value }
                    if field.trim().is_empty() || value.trim().is_empty() =>
                {
                    diagnostics.push(Diagnostic::error(
                        "AF036",
                        format!(
                            "Transition `{}` has an incomplete `datum_field_equals` constraint.",
                            transition.name
                        ),
                        "`datum_field_equals` requires both `field` and `value`.",
                    ));
                }
                Constraint::CustomAiken(value) if value.trim().is_empty() => {
                    diagnostics.push(Diagnostic::warning(
                        "AF037",
                        format!(
                            "Transition `{}` has an empty `custom_aiken` constraint.",
                            transition.name
                        ),
                        "Remove the empty constraint or provide an Aiken expression.",
                    ));
                }
                Constraint::Unknown { name, .. } if name.trim().is_empty() => {
                    diagnostics.push(Diagnostic::error(
                        "AF038",
                        format!(
                            "Transition `{}` has an unknown constraint with an empty name.",
                            transition.name
                        ),
                        "Use a named constraint key.",
                    ));
                }
                _ => {}
            }
        }
    }

    fn validate_output_assignments(
        transition: &TransitionDecl,
        output: &StateOutput,
        asset_names: &BTreeSet<String>,
        state_field_names: &BTreeMap<String, BTreeSet<String>>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for (asset, amount) in &output.value {
            if asset.trim().is_empty() || amount.trim().is_empty() {
                diagnostics.push(Diagnostic::error(
                    "AF029",
                    format!(
                        "Transition `{}` has an incomplete value assignment for output `{}`.",
                        transition.name, output.state
                    ),
                    "Every output value entry must include a non-empty asset name and amount expression.",
                ));
                continue;
            }

            if !asset_names.contains(asset) {
                diagnostics.push(Diagnostic::error(
                    "AF051",
                    format!(
                        "Transition `{}` output `{}` assigns undeclared asset `{}`.",
                        transition.name, output.state, asset
                    ),
                    "Declare the asset in top-level `assets` before using it in `outputs[].value`.",
                ));
            }
        }

        let known_fields = state_field_names.get(&output.state);
        for (field, value) in &output.datum {
            if field.trim().is_empty() || value.trim().is_empty() {
                diagnostics.push(Diagnostic::error(
                    "AF039",
                    format!(
                        "Transition `{}` has an incomplete datum assignment for output `{}`.",
                        transition.name, output.state
                    ),
                    "Every output datum entry must include a non-empty field name and value expression.",
                ));
                continue;
            }

            if let Some(known_fields) = known_fields {
                if !known_fields.contains(field) {
                    diagnostics.push(Diagnostic::error(
                        "AF044",
                        format!(
                            "Transition `{}` writes unknown datum field `{}` for output state `{}`.",
                            transition.name, field, output.state
                        ),
                        "Output datum assignments must target fields declared on the produced state.",
                    ));
                }
            }
        }
    }

    fn validate_asset_effects(
        transition: &TransitionDecl,
        effect_kind: &str,
        effects: &[AssetEffect],
        asset_names: &BTreeSet<String>,
        asset_kinds: &BTreeMap<String, &str>,
        asset_policies: &BTreeMap<String, Option<&str>>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for effect in effects {
            if effect.policy.trim().is_empty()
                || effect.asset.trim().is_empty()
                || effect.amount.trim().is_empty()
            {
                diagnostics.push(Diagnostic::error(
                    "AF052",
                    format!(
                        "Transition `{}` has an incomplete {effect_kind} effect.",
                        transition.name
                    ),
                    "Every mint/burn effect must include non-empty `policy`, `asset`, and `amount` fields.",
                ));
                continue;
            }

            if !asset_names.contains(&effect.asset) {
                diagnostics.push(Diagnostic::error(
                    "AF053",
                    format!(
                        "Transition `{}` {effect_kind}s undeclared asset `{}`.",
                        transition.name, effect.asset
                    ),
                    "Declare the asset in top-level `assets` before using it in `mints` or `burns`.",
                ));
                continue;
            }

            if asset_kinds
                .get(&effect.asset)
                .is_some_and(|kind| is_lovelace_kind(kind))
            {
                diagnostics.push(Diagnostic::error(
                    "AF058",
                    format!(
                        "Transition `{}` {effect_kind}s lovelace asset `{}`.",
                        transition.name, effect.asset
                    ),
                    "Lovelace cannot be minted or burned by protocol mint effects. Model ADA movement with inputs and outputs instead.",
                ));
            }

            if let Some(Some(expected_policy)) = asset_policies.get(&effect.asset) {
                if *expected_policy != effect.policy {
                    diagnostics.push(Diagnostic::error(
                        "AF054",
                        format!(
                            "Transition `{}` {effect_kind}s asset `{}` with policy `{}` but the asset declares policy `{}`.",
                            transition.name, effect.asset, effect.policy, expected_policy
                        ),
                        "Use the declared asset policy or update the top-level asset declaration.",
                    ));
                }
            }
        }
    }

    fn validate_input_bindings(
        transition: &TransitionDecl,
        inputs: &[StateInput],
        input_kind: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut seen = BTreeMap::<String, String>::new();

        for input in inputs {
            if input
                .alias
                .as_ref()
                .is_some_and(|alias| alias.trim().is_empty())
            {
                diagnostics.push(Diagnostic::error(
                    "AF045",
                    format!(
                        "Transition `{}` has a {input_kind} with an empty alias.",
                        transition.name
                    ),
                    "Remove the alias or set it to a stable name used for generated builder parameters.",
                ));
                continue;
            }

            if let Some(alias) = &input.alias {
                if !is_codegen_safe_name(alias) {
                    diagnostics.push(Diagnostic::error(
                        "AF065",
                        format!(
                            "Transition `{}` has a {input_kind} alias `{alias}` that is not safe for generated identifiers.",
                            transition.name
                        ),
                        "Use an alias that normalizes to an identifier beginning with an ASCII letter and is not a reserved word.",
                    ));
                    continue;
                }
            }

            let raw_binding = input.alias.as_deref().unwrap_or(input.state.as_str());
            let binding = to_camel_case(raw_binding);
            if binding.is_empty() {
                continue;
            }

            if let Some(previous) = seen.insert(binding.clone(), raw_binding.to_owned()) {
                diagnostics.push(Diagnostic::error(
                    "AF046",
                    format!(
                        "Transition `{}` has duplicate {input_kind} binding `{}`.",
                        transition.name, binding
                    ),
                    format!(
                        "`{previous}` and `{raw_binding}` resolve to the same generated builder parameter prefix. Add distinct aliases."
                    ),
                ));
            }
        }
    }

    fn validate_output_bindings(transition: &TransitionDecl, diagnostics: &mut Vec<Diagnostic>) {
        let mut output_bindings = BTreeMap::<String, String>::new();

        for output in &transition.outputs {
            let binding = to_camel_case(&output.state);
            if binding.is_empty() {
                continue;
            }

            if let Some(previous) = output_bindings.insert(binding.clone(), output.state.clone()) {
                diagnostics.push(Diagnostic::error(
                    "AF060",
                    format!(
                        "Transition `{}` has duplicate output binding `{}`.",
                        transition.name, binding
                    ),
                    format!(
                        "Output states `{previous}` and `{}` resolve to the same generated builder parameter prefix. Split the transition or introduce explicit output aliases in a future spec version.",
                        output.state
                    ),
                ));
            }
        }

        for input in &transition.inputs {
            let input_binding = input_binding_name(input);
            if input_binding.is_empty() {
                continue;
            }

            for output in &transition.outputs {
                if input_binding == to_camel_case(&output.state) && input.state != output.state {
                    diagnostics.push(Diagnostic::error(
                        "AF059",
                        format!(
                            "Transition `{}` input binding `{}` collides with output state `{}`.",
                            transition.name, input_binding, output.state
                        ),
                        "Use an input alias that does not resolve to the produced state's generated parameter prefix.",
                    ));
                }
            }
        }
    }

    fn validate_constraint_type_compatibility(
        &self,
        transition: &TransitionDecl,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for constraint in &transition.constraints {
            match constraint {
                Constraint::SignedBy(expr) => {
                    self.validate_resolved_field_type(
                        transition,
                        expr,
                        FieldTypeRule {
                            constraint_name: "signed_by",
                            accepts_type: is_signature_type,
                            code: "AF047",
                            hint: "Signature constraints must target key-hash-like datum fields.",
                        },
                        diagnostics,
                    );
                }
                Constraint::Before(expr) | Constraint::After(expr) => {
                    self.validate_resolved_field_type(
                        transition,
                        expr,
                        FieldTypeRule {
                            constraint_name: "validity",
                            accepts_type: is_numeric_type,
                            code: "AF048",
                            hint:
                                "Validity constraints must target POSIXTime/Int-like datum fields.",
                        },
                        diagnostics,
                    );
                }
                Constraint::Positive(expr) => {
                    self.validate_resolved_field_type(
                        transition,
                        expr,
                        FieldTypeRule {
                            constraint_name: "positive",
                            accepts_type: is_numeric_type,
                            code: "AF049",
                            hint: "Positive constraints must target numeric datum fields.",
                        },
                        diagnostics,
                    );
                }
                Constraint::DatumFieldEquals { field, value } => {
                    self.validate_datum_field_equals_compatibility(
                        transition,
                        field,
                        value,
                        diagnostics,
                    );
                }
                Constraint::ValuePreserved(_)
                | Constraint::OutputExists(_)
                | Constraint::CustomAiken(_)
                | Constraint::Unknown { .. } => {}
            }
        }
    }

    fn validate_resolved_field_type(
        &self,
        transition: &TransitionDecl,
        expr: &str,
        rule: FieldTypeRule,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for input in &transition.inputs {
            let Some(state) = self.state(&input.state) else {
                continue;
            };
            let Some(field) = resolve_field_ref(state, expr) else {
                continue;
            };

            if !(rule.accepts_type)(&field.ty) {
                diagnostics.push(Diagnostic::error(
                    rule.code,
                    format!(
                        "Transition `{}` uses `{}` on `{}` field `{}` with incompatible type `{}`.",
                        transition.name, rule.constraint_name, state.name, field.name, field.ty
                    ),
                    rule.hint,
                ));
            }
        }
    }

    fn validate_datum_field_equals_compatibility(
        &self,
        transition: &TransitionDecl,
        field_expr: &str,
        value_expr: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if transition.inputs.is_empty() {
            return;
        }

        let mut resolved_any_target = false;
        for input in &transition.inputs {
            let Some(state) = self.state(&input.state) else {
                continue;
            };
            let Some(target_field) = resolve_field_ref(state, field_expr) else {
                continue;
            };
            resolved_any_target = true;

            if let Some(value_field) = resolve_field_ref(state, value_expr) {
                if !datum_types_compatible_for_equality(&target_field.ty, &value_field.ty) {
                    diagnostics.push(Diagnostic::error(
                        "AF067",
                        format!(
                            "Transition `{}` compares `{}` field `{}` of type `{}` with field `{}` of incompatible type `{}`.",
                            transition.name,
                            state.name,
                            target_field.name,
                            target_field.ty,
                            value_field.name,
                            value_field.ty
                        ),
                        "`datum_field_equals` comparisons must use compatible datum field types.",
                    ));
                }
                continue;
            }

            if !literal_compatible_with_type(value_expr, &target_field.ty) {
                diagnostics.push(Diagnostic::error(
                    "AF068",
                    format!(
                        "Transition `{}` compares `{}` field `{}` of type `{}` with incompatible literal `{}`.",
                        transition.name, state.name, target_field.name, target_field.ty, value_expr
                    ),
                    "`datum_field_equals` literals must match the target field type.",
                ));
            }
        }

        if !resolved_any_target {
            diagnostics.push(Diagnostic::error(
                "AF066",
                format!(
                    "Transition `{}` uses `datum_field_equals` on unresolved field `{}`.",
                    transition.name, field_expr
                ),
                "`datum_field_equals.field` must resolve to a datum field on a consumed state.",
            ));
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FieldTypeRule {
    constraint_name: &'static str,
    accepts_type: fn(&str) -> bool,
    code: &'static str,
    hint: &'static str,
}

fn resolve_field_ref<'a>(state: &'a StateDecl, expr: &str) -> Option<&'a FieldDecl> {
    let trimmed = expr.trim().trim_matches('"');
    let candidate = trimmed
        .strip_prefix("datum.")
        .or_else(|| trimmed.strip_prefix('$'))
        .unwrap_or(trimmed);

    state.datum.iter().find(|field| field.name == candidate)
}

fn input_binding_name(input: &StateInput) -> String {
    input
        .alias
        .as_deref()
        .map(to_camel_case)
        .filter(|alias| !alias.is_empty())
        .unwrap_or_else(|| to_camel_case(&input.state))
}

fn is_codegen_safe_name(name: &str) -> bool {
    let normalized = to_snake_case(name);

    normalized
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && !is_reserved_codegen_identifier(&normalized)
}

fn is_reserved_codegen_identifier(identifier: &str) -> bool {
    matches!(
        identifier,
        "as" | "async"
            | "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "expect"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "fn"
            | "for"
            | "from"
            | "function"
            | "if"
            | "implements"
            | "import"
            | "in"
            | "instanceof"
            | "interface"
            | "is"
            | "let"
            | "new"
            | "opaque"
            | "package"
            | "private"
            | "protected"
            | "pub"
            | "public"
            | "return"
            | "static"
            | "super"
            | "switch"
            | "throw"
            | "todo"
            | "trace"
            | "true"
            | "try"
            | "type"
            | "typeof"
            | "use"
            | "validator"
            | "var"
            | "via"
            | "void"
            | "when"
            | "while"
            | "with"
            | "yield"
    )
}

fn is_signature_type(ty: &str) -> bool {
    matches!(ty.trim(), "PubKeyHash" | "VerificationKeyHash" | "VKeyHash")
}

fn is_numeric_type(ty: &str) -> bool {
    matches!(
        ty.trim(),
        "Lovelace" | "POSIXTime" | "Integer" | "Int" | "Natural"
    )
}

fn is_bool_type(ty: &str) -> bool {
    matches!(ty.trim(), "Bool" | "Boolean")
}

fn is_bytearray_type(ty: &str) -> bool {
    matches!(ty.trim(), "ByteArray" | "Bytes")
}

fn datum_types_compatible_for_equality(left: &str, right: &str) -> bool {
    let left = left.trim();
    let right = right.trim();

    left == right
        || (is_signature_type(left) && is_signature_type(right))
        || (is_numeric_type(left) && is_numeric_type(right))
        || (is_bool_type(left) && is_bool_type(right))
        || (is_bytearray_type(left) && is_bytearray_type(right))
}

fn literal_compatible_with_type(value: &str, ty: &str) -> bool {
    let value = value.trim();
    let ty = ty.trim();

    if value.parse::<i64>().is_ok() {
        return is_numeric_type(ty);
    }

    if matches!(value, "True" | "False" | "true" | "false") {
        return is_bool_type(ty);
    }

    is_bytearray_type(ty) || ty == "Data"
}

fn is_lovelace_kind(kind: &str) -> bool {
    kind.trim().eq_ignore_ascii_case("lovelace")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetDecl {
    pub name: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateDecl {
    pub name: String,
    #[serde(default)]
    pub datum: DatumSchema,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    #[serde(default)]
    pub terminal: bool,
}

pub type DatumSchema = Vec<FieldDecl>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDecl {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionDecl {
    pub name: String,
    #[serde(default)]
    pub inputs: Vec<StateInput>,
    #[serde(default)]
    pub reference_inputs: Vec<StateInput>,
    #[serde(default)]
    pub outputs: Vec<StateOutput>,
    #[serde(default)]
    pub mints: Vec<AssetEffect>,
    #[serde(default)]
    pub burns: Vec<AssetEffect>,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    #[serde(default)]
    pub effects: Vec<Effect>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateInput {
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateOutput {
    pub state: String,
    #[serde(default)]
    pub value: BTreeMap<String, String>,
    #[serde(default)]
    pub datum: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetEffect {
    pub policy: String,
    pub asset: String,
    pub amount: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Constraint {
    SignedBy(String),
    Before(String),
    After(String),
    Positive(String),
    ValuePreserved(Option<String>),
    OutputExists(String),
    DatumFieldEquals { field: String, value: String },
    CustomAiken(String),
    Unknown { name: String, value: Option<String> },
}

impl Constraint {
    pub fn label(&self) -> String {
        match self {
            Constraint::SignedBy(value) => format!("signed_by({value})"),
            Constraint::Before(value) => format!("before({value})"),
            Constraint::After(value) => format!("after({value})"),
            Constraint::Positive(value) => format!("positive({value})"),
            Constraint::ValuePreserved(Some(value)) => format!("value_preserved({value})"),
            Constraint::ValuePreserved(None) => "value_preserved".to_owned(),
            Constraint::OutputExists(value) => format!("output_exists({value})"),
            Constraint::DatumFieldEquals { field, value } => {
                format!("datum_field_equals({field}, {value})")
            }
            Constraint::CustomAiken(value) => format!("custom_aiken({value})"),
            Constraint::Unknown { name, value } => match value {
                Some(value) => format!("{name}({value})"),
                None => name.clone(),
            },
        }
    }

    pub fn signature_requirement(&self) -> Option<&str> {
        match self {
            Constraint::SignedBy(value) => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn validity_requirement(&self) -> Option<(&'static str, &str)> {
        match self {
            Constraint::Before(value) => Some(("before", value.as_str())),
            Constraint::After(value) => Some(("after", value.as_str())),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    CreateState(StateOutput),
    UpdateDatum { field: String, value: String },
    MintAsset(AssetEffect),
    BurnAsset(AssetEffect),
    Unknown { name: String, value: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvariantDecl {
    pub name: String,
    pub expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionTopology {
    pub transition: String,
    pub consumes: Vec<String>,
    pub references: Vec<String>,
    pub produces: Vec<String>,
    pub mints: Vec<String>,
    pub burns: Vec<String>,
    pub requires_signatures: Vec<String>,
    pub requires_validity: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

impl GeneratedFile {
    pub fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedFiles {
    pub files: Vec<GeneratedFile>,
}

impl GeneratedFiles {
    pub fn new(files: Vec<GeneratedFile>) -> Self {
        Self { files }
    }

    pub fn push(&mut self, path: impl Into<String>, content: impl Into<String>) {
        self.files.push(GeneratedFile::new(path, content));
    }

    pub fn extend(&mut self, other: GeneratedFiles) {
        self.files.extend(other.files);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticSeverity::Error => f.write_str("error"),
            DiagnosticSeverity::Warning => f.write_str("warning"),
            DiagnosticSeverity::Info => f.write_str("info"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub hint: String,
}

impl Diagnostic {
    pub fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        Self::new(DiagnosticSeverity::Error, code, message, hint)
    }

    pub fn warning(
        code: impl Into<String>,
        message: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        Self::new(DiagnosticSeverity::Warning, code, message, hint)
    }

    pub fn info(
        code: impl Into<String>,
        message: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        Self::new(DiagnosticSeverity::Info, code, message, hint)
    }

    fn new(
        severity: DiagnosticSeverity,
        code: impl Into<String>,
        message: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            hint: hint.into(),
        }
    }
}

pub fn to_snake_case(input: &str) -> String {
    let mut output = String::new();
    let mut previous_was_separator = true;

    for (index, character) in input.chars().enumerate() {
        if character.is_ascii_alphanumeric() {
            if character.is_ascii_uppercase() && index > 0 && !previous_was_separator {
                output.push('_');
            }
            output.push(character.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            output.push('_');
            previous_was_separator = true;
        }
    }

    output.trim_matches('_').to_owned()
}

pub fn to_pascal_case(input: &str) -> String {
    let mut output = String::new();
    for part in to_snake_case(input).split('_') {
        if part.is_empty() {
            continue;
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            output.push(first.to_ascii_uppercase());
            output.extend(chars);
        }
    }
    output
}

pub fn to_camel_case(input: &str) -> String {
    let pascal = to_pascal_case(input);
    let mut chars = pascal.chars();
    match chars.next() {
        Some(first) => {
            let mut output = String::new();
            output.push(first.to_ascii_lowercase());
            output.extend(chars);
            output
        }
        None => String::new(),
    }
}

pub fn sanitize_aiken_identifier(input: &str) -> String {
    let snake = to_snake_case(input);
    if snake.is_empty() {
        "unnamed".to_owned()
    } else {
        snake
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_protocol() -> Protocol {
        Protocol {
            name: "Minimal".to_owned(),
            description: None,
            assets: vec![],
            states: vec![StateDecl {
                name: "Locked".to_owned(),
                datum: vec![FieldDecl {
                    name: "owner".to_owned(),
                    ty: "PubKeyHash".to_owned(),
                }],
                script: None,
                terminal: false,
            }],
            transitions: vec![],
            invariants: vec![InvariantDecl {
                name: "owner_controls_spend".to_owned(),
                expression: "spend requires owner".to_owned(),
            }],
        }
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn validates_unknown_state_references() {
        let protocol = Protocol {
            name: "Broken".to_owned(),
            description: None,
            assets: vec![],
            states: vec![StateDecl {
                name: "Locked".to_owned(),
                datum: vec![],
                script: None,
                terminal: false,
            }],
            transitions: vec![TransitionDecl {
                name: "Withdraw".to_owned(),
                inputs: vec![StateInput {
                    state: "Missing".to_owned(),
                    alias: None,
                }],
                reference_inputs: vec![],
                outputs: vec![],
                mints: vec![],
                burns: vec![],
                constraints: vec![],
                effects: vec![],
            }],
            invariants: vec![],
        };

        let diagnostics = protocol.validate();

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "AF024"));
    }

    #[test]
    fn validates_asset_declarations() {
        let mut protocol = minimal_protocol();
        protocol.assets = vec![
            AssetDecl {
                name: "ada".to_owned(),
                kind: "lovelace".to_owned(),
                policy: None,
            },
            AssetDecl {
                name: "ada".to_owned(),
                kind: String::new(),
                policy: None,
            },
        ];

        let diagnostics = protocol.validate();

        assert!(has_code(&diagnostics, "AF011"));
        assert!(has_code(&diagnostics, "AF012"));
    }

    #[test]
    fn validates_asset_references() {
        let mut protocol = minimal_protocol();
        protocol.assets = vec![
            AssetDecl {
                name: "regulated_token".to_owned(),
                kind: "native".to_owned(),
                policy: Some("regulated_policy".to_owned()),
            },
            AssetDecl {
                name: "ada".to_owned(),
                kind: "lovelace".to_owned(),
                policy: None,
            },
        ];
        protocol.transitions = vec![TransitionDecl {
            name: "Issue".to_owned(),
            inputs: vec![],
            reference_inputs: vec![],
            outputs: vec![StateOutput {
                state: "Locked".to_owned(),
                value: [("missing_token".to_owned(), "1".to_owned())].into(),
                datum: BTreeMap::new(),
            }],
            mints: vec![
                AssetEffect {
                    policy: "other_policy".to_owned(),
                    asset: "regulated_token".to_owned(),
                    amount: "1".to_owned(),
                },
                AssetEffect {
                    policy: "regulated_policy".to_owned(),
                    asset: "unknown_token".to_owned(),
                    amount: "1".to_owned(),
                },
                AssetEffect {
                    policy: String::new(),
                    asset: "regulated_token".to_owned(),
                    amount: "1".to_owned(),
                },
                AssetEffect {
                    policy: "ada_policy".to_owned(),
                    asset: "ada".to_owned(),
                    amount: "1".to_owned(),
                },
            ],
            burns: vec![],
            constraints: vec![],
            effects: vec![],
        }];

        let diagnostics = protocol.validate();

        for code in ["AF051", "AF052", "AF053", "AF054", "AF058"] {
            assert!(has_code(&diagnostics, code), "missing {code}");
        }
    }

    #[test]
    fn validates_empty_datum_types() {
        let mut protocol = minimal_protocol();
        protocol.states[0].datum[0].ty = String::new();

        let diagnostics = protocol.validate();

        assert!(has_code(&diagnostics, "AF007"));
    }

    #[test]
    fn validates_generated_identifier_collisions() {
        let protocol = Protocol {
            name: "Collisions".to_owned(),
            description: None,
            assets: vec![],
            states: vec![
                StateDecl {
                    name: "LockedState".to_owned(),
                    datum: vec![
                        FieldDecl {
                            name: "highestBid".to_owned(),
                            ty: "Int".to_owned(),
                        },
                        FieldDecl {
                            name: "highest_bid".to_owned(),
                            ty: "Int".to_owned(),
                        },
                    ],
                    script: None,
                    terminal: false,
                },
                StateDecl {
                    name: "locked-state".to_owned(),
                    datum: vec![],
                    script: None,
                    terminal: false,
                },
            ],
            transitions: vec![
                TransitionDecl {
                    name: "CloseAuction".to_owned(),
                    inputs: vec![],
                    reference_inputs: vec![],
                    outputs: vec![],
                    mints: vec![],
                    burns: vec![],
                    constraints: vec![],
                    effects: vec![],
                },
                TransitionDecl {
                    name: "close_auction".to_owned(),
                    inputs: vec![],
                    reference_inputs: vec![],
                    outputs: vec![],
                    mints: vec![],
                    burns: vec![],
                    constraints: vec![],
                    effects: vec![],
                },
            ],
            invariants: vec![InvariantDecl {
                name: "reviewed".to_owned(),
                expression: "manual review".to_owned(),
            }],
        };

        let diagnostics = protocol.validate();

        for code in ["AF055", "AF056", "AF057"] {
            assert!(has_code(&diagnostics, code), "missing {code}");
        }
    }

    #[test]
    fn validates_codegen_safe_names() {
        let protocol = Protocol {
            name: "123Protocol".to_owned(),
            description: None,
            assets: vec![],
            states: vec![StateDecl {
                name: "!!!".to_owned(),
                datum: vec![FieldDecl {
                    name: "123-owner".to_owned(),
                    ty: "PubKeyHash".to_owned(),
                }],
                script: None,
                terminal: false,
            }],
            transitions: vec![TransitionDecl {
                name: "123Spend".to_owned(),
                inputs: vec![StateInput {
                    state: "!!!".to_owned(),
                    alias: Some("123-input".to_owned()),
                }],
                reference_inputs: vec![],
                outputs: vec![],
                mints: vec![],
                burns: vec![],
                constraints: vec![],
                effects: vec![],
            }],
            invariants: vec![InvariantDecl {
                name: "reviewed".to_owned(),
                expression: "manual review".to_owned(),
            }],
        };

        let diagnostics = protocol.validate();

        for code in ["AF061", "AF062", "AF063", "AF064", "AF065"] {
            assert!(has_code(&diagnostics, code), "missing {code}");
        }
    }

    #[test]
    fn validates_reserved_codegen_names() {
        let protocol = Protocol {
            name: "type".to_owned(),
            description: None,
            assets: vec![],
            states: vec![StateDecl {
                name: "validator".to_owned(),
                datum: vec![FieldDecl {
                    name: "return".to_owned(),
                    ty: "PubKeyHash".to_owned(),
                }],
                script: None,
                terminal: false,
            }],
            transitions: vec![TransitionDecl {
                name: "function".to_owned(),
                inputs: vec![StateInput {
                    state: "validator".to_owned(),
                    alias: Some("class".to_owned()),
                }],
                reference_inputs: vec![],
                outputs: vec![],
                mints: vec![],
                burns: vec![],
                constraints: vec![],
                effects: vec![],
            }],
            invariants: vec![InvariantDecl {
                name: "reviewed".to_owned(),
                expression: "manual review".to_owned(),
            }],
        };

        let diagnostics = protocol.validate();

        for code in ["AF061", "AF062", "AF063", "AF064", "AF065"] {
            assert!(has_code(&diagnostics, code), "missing {code}");
        }
    }

    #[test]
    fn rejects_consuming_terminal_states() {
        let mut protocol = minimal_protocol();
        protocol.states[0].terminal = true;
        protocol.transitions = vec![TransitionDecl {
            name: "Spend".to_owned(),
            inputs: vec![StateInput {
                state: "Locked".to_owned(),
                alias: None,
            }],
            reference_inputs: vec![],
            outputs: vec![],
            mints: vec![],
            burns: vec![],
            constraints: vec![Constraint::SignedBy("datum.owner".to_owned())],
            effects: vec![],
        }];

        let diagnostics = protocol.validate();

        assert!(has_code(&diagnostics, "AF027"));
    }

    #[test]
    fn validates_empty_constraint_values() {
        let mut protocol = minimal_protocol();
        protocol.transitions = vec![TransitionDecl {
            name: "Spend".to_owned(),
            inputs: vec![StateInput {
                state: "Locked".to_owned(),
                alias: None,
            }],
            reference_inputs: vec![],
            outputs: vec![],
            mints: vec![],
            burns: vec![],
            constraints: vec![
                Constraint::SignedBy(String::new()),
                Constraint::Before(String::new()),
                Constraint::After(String::new()),
                Constraint::Positive(String::new()),
                Constraint::DatumFieldEquals {
                    field: String::new(),
                    value: "owner".to_owned(),
                },
                Constraint::CustomAiken(String::new()),
            ],
            effects: vec![],
        }];

        let diagnostics = protocol.validate();

        for code in ["AF032", "AF033", "AF034", "AF035", "AF036", "AF037"] {
            assert!(has_code(&diagnostics, code), "missing {code}");
        }
    }

    #[test]
    fn validates_output_assignments() {
        let mut protocol = minimal_protocol();
        protocol.transitions = vec![TransitionDecl {
            name: "Update".to_owned(),
            inputs: vec![],
            reference_inputs: vec![],
            outputs: vec![StateOutput {
                state: "Locked".to_owned(),
                value: [("".to_owned(), "$amount".to_owned())].into(),
                datum: [
                    ("owner".to_owned(), String::new()),
                    ("missing".to_owned(), "$owner".to_owned()),
                ]
                .into(),
            }],
            mints: vec![],
            burns: vec![],
            constraints: vec![],
            effects: vec![],
        }];

        let diagnostics = protocol.validate();

        for code in ["AF029", "AF039", "AF044"] {
            assert!(has_code(&diagnostics, code), "missing {code}");
        }
    }

    #[test]
    fn warns_when_creation_output_assets_cannot_be_inferred() {
        let mut protocol = minimal_protocol();
        protocol.transitions = vec![TransitionDecl {
            name: "Create".to_owned(),
            inputs: vec![],
            reference_inputs: vec![],
            outputs: vec![StateOutput {
                state: "Locked".to_owned(),
                value: BTreeMap::new(),
                datum: BTreeMap::new(),
            }],
            mints: vec![],
            burns: vec![],
            constraints: vec![],
            effects: vec![],
        }];

        let diagnostics = protocol.validate();

        assert!(has_code(&diagnostics, "AF050"));
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "AF050" && diagnostic.severity == DiagnosticSeverity::Warning
        }));
    }

    #[test]
    fn validates_input_alias_bindings() {
        let mut protocol = minimal_protocol();
        protocol.transitions = vec![TransitionDecl {
            name: "SpendBoth".to_owned(),
            inputs: vec![
                StateInput {
                    state: "Locked".to_owned(),
                    alias: Some("".to_owned()),
                },
                StateInput {
                    state: "Locked".to_owned(),
                    alias: Some("left-input".to_owned()),
                },
                StateInput {
                    state: "Locked".to_owned(),
                    alias: Some("left_input".to_owned()),
                },
            ],
            reference_inputs: vec![
                StateInput {
                    state: "Locked".to_owned(),
                    alias: None,
                },
                StateInput {
                    state: "Locked".to_owned(),
                    alias: None,
                },
            ],
            outputs: vec![],
            mints: vec![],
            burns: vec![],
            constraints: vec![Constraint::SignedBy("datum.owner".to_owned())],
            effects: vec![],
        }];

        let diagnostics = protocol.validate();

        assert!(has_code(&diagnostics, "AF045"));
        assert!(has_code(&diagnostics, "AF046"));
    }

    #[test]
    fn validates_output_parameter_binding_collisions() {
        let mut protocol = minimal_protocol();
        protocol.states.push(StateDecl {
            name: "Closed".to_owned(),
            datum: vec![FieldDecl {
                name: "owner".to_owned(),
                ty: "PubKeyHash".to_owned(),
            }],
            script: None,
            terminal: false,
        });
        protocol.transitions = vec![
            TransitionDecl {
                name: "AliasCollision".to_owned(),
                inputs: vec![StateInput {
                    state: "Locked".to_owned(),
                    alias: Some("closed".to_owned()),
                }],
                reference_inputs: vec![],
                outputs: vec![StateOutput {
                    state: "Closed".to_owned(),
                    value: BTreeMap::new(),
                    datum: BTreeMap::new(),
                }],
                mints: vec![],
                burns: vec![],
                constraints: vec![Constraint::SignedBy("datum.owner".to_owned())],
                effects: vec![],
            },
            TransitionDecl {
                name: "DuplicateOutputs".to_owned(),
                inputs: vec![StateInput {
                    state: "Locked".to_owned(),
                    alias: None,
                }],
                reference_inputs: vec![],
                outputs: vec![
                    StateOutput {
                        state: "Closed".to_owned(),
                        value: BTreeMap::new(),
                        datum: BTreeMap::new(),
                    },
                    StateOutput {
                        state: "Closed".to_owned(),
                        value: BTreeMap::new(),
                        datum: BTreeMap::new(),
                    },
                ],
                mints: vec![],
                burns: vec![],
                constraints: vec![Constraint::SignedBy("datum.owner".to_owned())],
                effects: vec![],
            },
        ];

        let diagnostics = protocol.validate();

        assert!(has_code(&diagnostics, "AF059"));
        assert!(has_code(&diagnostics, "AF060"));
    }

    #[test]
    fn validates_constraint_type_compatibility() {
        let mut protocol = minimal_protocol();
        protocol.states[0].datum = vec![
            FieldDecl {
                name: "owner".to_owned(),
                ty: "Int".to_owned(),
            },
            FieldDecl {
                name: "deadline".to_owned(),
                ty: "PubKeyHash".to_owned(),
            },
            FieldDecl {
                name: "amount".to_owned(),
                ty: "ByteArray".to_owned(),
            },
            FieldDecl {
                name: "raw_owner".to_owned(),
                ty: "ByteArray".to_owned(),
            },
        ];
        protocol.transitions = vec![TransitionDecl {
            name: "Spend".to_owned(),
            inputs: vec![StateInput {
                state: "Locked".to_owned(),
                alias: None,
            }],
            reference_inputs: vec![],
            outputs: vec![],
            mints: vec![],
            burns: vec![],
            constraints: vec![
                Constraint::SignedBy("datum.owner".to_owned()),
                Constraint::SignedBy("datum.raw_owner".to_owned()),
                Constraint::After("datum.deadline".to_owned()),
                Constraint::Positive("datum.amount".to_owned()),
            ],
            effects: vec![],
        }];

        let diagnostics = protocol.validate();

        for code in ["AF047", "AF048", "AF049"] {
            assert!(has_code(&diagnostics, code), "missing {code}");
        }
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == "AF047")
                .count(),
            2
        );
    }

    #[test]
    fn validates_datum_field_equals_type_compatibility() {
        let mut protocol = minimal_protocol();
        protocol.states[0].datum = vec![
            FieldDecl {
                name: "owner".to_owned(),
                ty: "PubKeyHash".to_owned(),
            },
            FieldDecl {
                name: "amount".to_owned(),
                ty: "Int".to_owned(),
            },
            FieldDecl {
                name: "enabled".to_owned(),
                ty: "Bool".to_owned(),
            },
        ];
        protocol.transitions = vec![TransitionDecl {
            name: "Spend".to_owned(),
            inputs: vec![StateInput {
                state: "Locked".to_owned(),
                alias: None,
            }],
            reference_inputs: vec![],
            outputs: vec![],
            mints: vec![],
            burns: vec![],
            constraints: vec![
                Constraint::DatumFieldEquals {
                    field: "datum.missing".to_owned(),
                    value: "1".to_owned(),
                },
                Constraint::DatumFieldEquals {
                    field: "datum.amount".to_owned(),
                    value: "datum.owner".to_owned(),
                },
                Constraint::DatumFieldEquals {
                    field: "datum.enabled".to_owned(),
                    value: "1".to_owned(),
                },
            ],
            effects: vec![],
        }];

        let diagnostics = protocol.validate();

        for code in ["AF066", "AF067", "AF068"] {
            assert!(has_code(&diagnostics, code), "missing {code}");
        }
    }

    #[test]
    fn validates_invariant_declarations() {
        let mut protocol = minimal_protocol();
        protocol.invariants = vec![
            InvariantDecl {
                name: "stable".to_owned(),
                expression: "something holds".to_owned(),
            },
            InvariantDecl {
                name: "stable".to_owned(),
                expression: String::new(),
            },
        ];

        let diagnostics = protocol.validate();

        assert!(has_code(&diagnostics, "AF042"));
        assert!(has_code(&diagnostics, "AF043"));
    }

    #[test]
    fn converts_names_to_codegen_cases() {
        assert_eq!(to_snake_case("SimpleVault"), "simple_vault");
        assert_eq!(to_pascal_case("simple-vault"), "SimpleVault");
        assert_eq!(to_camel_case("Withdraw From Vault"), "withdrawFromVault");
    }
}

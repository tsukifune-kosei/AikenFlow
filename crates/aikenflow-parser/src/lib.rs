use aikenflow_ir::{
    AssetDecl, AssetEffect, Constraint, FieldDecl, InvariantDecl, Protocol, StateDecl, StateInput,
    StateOutput, TransitionDecl,
};
use serde::de::{self, MapAccess, Visitor};
use serde::Deserialize;
use serde_yaml::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::marker::PhantomData;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("failed to read `{path}`: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid protocol YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("invalid protocol spec: {0}")]
    Invalid(String),
}

pub fn parse_protocol_file(path: impl AsRef<Path>) -> Result<Protocol, ParseError> {
    let path = path.as_ref();
    let source = fs::read_to_string(path).map_err(|source| ParseError::Read {
        path: path.display().to_string(),
        source,
    })?;
    parse_protocol(&source)
}

pub fn parse_protocol(source: &str) -> Result<Protocol, ParseError> {
    let spec: ProtocolSpec = serde_yaml::from_str(source)?;
    spec.try_into()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtocolSpec {
    #[serde(alias = "name")]
    protocol: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    assets: Vec<AssetSpec>,
    #[serde(default)]
    states: NamedMap<StateSpec>,
    #[serde(default)]
    transitions: NamedMap<TransitionSpec>,
    #[serde(default)]
    invariants: Vec<InvariantSpec>,
}

impl TryFrom<ProtocolSpec> for Protocol {
    type Error = ParseError;

    fn try_from(spec: ProtocolSpec) -> Result<Self, Self::Error> {
        let states = spec
            .states
            .into_entries()
            .into_iter()
            .map(|(name, state)| state.into_state(name))
            .collect::<Result<Vec<_>, _>>()?;

        let transitions = spec
            .transitions
            .into_entries()
            .into_iter()
            .map(|(name, transition)| transition.into_transition(name))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Protocol {
            name: spec.protocol,
            description: spec.description,
            assets: spec.assets.into_iter().map(AssetSpec::into_asset).collect(),
            states,
            transitions,
            invariants: spec
                .invariants
                .into_iter()
                .map(InvariantSpec::into_invariant)
                .collect(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetSpec {
    name: String,
    #[serde(default = "default_asset_kind")]
    kind: String,
    #[serde(default)]
    policy: Option<String>,
}

impl AssetSpec {
    fn into_asset(self) -> AssetDecl {
        AssetDecl {
            name: self.name,
            kind: self.kind,
            policy: self.policy,
        }
    }
}

fn default_asset_kind() -> String {
    "native".to_owned()
}

#[derive(Debug)]
struct NamedMap<T>(Vec<(String, T)>);

impl<T> NamedMap<T> {
    fn into_entries(self) -> Vec<(String, T)> {
        self.0
    }
}

impl<T> Default for NamedMap<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<'de, T> Deserialize<'de> for NamedMap<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(NamedMapVisitor {
            marker: PhantomData,
        })
    }
}

struct NamedMapVisitor<T> {
    marker: PhantomData<T>,
}

impl<'de, T> Visitor<'de> for NamedMapVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = NamedMap<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a YAML map with unique string keys")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut entries = Vec::new();
        let mut seen = BTreeSet::new();

        while let Some((key, value)) = access.next_entry::<String, T>()? {
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom(format!("duplicate declaration `{key}`")));
            }
            entries.push((key, value));
        }

        Ok(NamedMap(entries))
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StateSpec {
    #[serde(default)]
    datum: NamedMap<String>,
    #[serde(default)]
    script: Option<String>,
    #[serde(default)]
    terminal: bool,
}

impl StateSpec {
    fn into_state(self, name: String) -> Result<StateDecl, ParseError> {
        Ok(StateDecl {
            name,
            datum: self
                .datum
                .into_entries()
                .into_iter()
                .map(|(name, ty)| FieldDecl { name, ty })
                .collect(),
            script: self.script,
            terminal: self.terminal,
        })
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct TransitionSpec {
    #[serde(default)]
    inputs: Vec<StateRefSpec>,
    #[serde(default)]
    reference_inputs: Vec<StateRefSpec>,
    #[serde(default)]
    outputs: Vec<StateOutputSpec>,
    #[serde(default)]
    mints: Vec<AssetEffectSpec>,
    #[serde(default)]
    burns: Vec<AssetEffectSpec>,
    #[serde(default)]
    constraints: Vec<Value>,
}

impl TransitionSpec {
    fn into_transition(self, name: String) -> Result<TransitionDecl, ParseError> {
        Ok(TransitionDecl {
            name,
            inputs: self
                .inputs
                .into_iter()
                .map(StateRefSpec::into_state_input)
                .collect(),
            reference_inputs: self
                .reference_inputs
                .into_iter()
                .map(StateRefSpec::into_state_input)
                .collect(),
            outputs: self
                .outputs
                .into_iter()
                .map(StateOutputSpec::into_state_output)
                .collect::<Result<Vec<_>, _>>()?,
            mints: self
                .mints
                .into_iter()
                .map(AssetEffectSpec::into_asset_effect)
                .collect(),
            burns: self
                .burns
                .into_iter()
                .map(AssetEffectSpec::into_asset_effect)
                .collect(),
            constraints: self
                .constraints
                .iter()
                .map(parse_constraint)
                .collect::<Result<Vec<_>, _>>()?,
            effects: vec![],
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StateRefSpec {
    Name(String),
    Object(StateRefObject),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StateRefObject {
    state: String,
    #[serde(default)]
    alias: Option<String>,
}

impl StateRefSpec {
    fn into_state_input(self) -> StateInput {
        match self {
            StateRefSpec::Name(state) => StateInput { state, alias: None },
            StateRefSpec::Object(object) => StateInput {
                state: object.state,
                alias: object.alias,
            },
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StateOutputSpec {
    state: String,
    #[serde(default)]
    value: NamedMap<Value>,
    #[serde(default)]
    datum: NamedMap<Value>,
}

impl StateOutputSpec {
    fn into_state_output(self) -> Result<StateOutput, ParseError> {
        Ok(StateOutput {
            state: self.state,
            value: value_map_to_string_map(self.value)?,
            datum: value_map_to_string_map(self.datum)?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetEffectSpec {
    policy: String,
    asset: String,
    amount: Value,
}

impl AssetEffectSpec {
    fn into_asset_effect(self) -> AssetEffect {
        AssetEffect {
            policy: self.policy,
            asset: self.asset,
            amount: scalar_to_string(&self.amount),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InvariantSpec {
    name: String,
    expression: String,
}

impl InvariantSpec {
    fn into_invariant(self) -> InvariantDecl {
        InvariantDecl {
            name: self.name,
            expression: self.expression,
        }
    }
}

fn parse_constraint(value: &Value) -> Result<Constraint, ParseError> {
    match value {
        Value::String(text) => Ok(Constraint::CustomAiken(text.clone())),
        Value::Mapping(mapping) => {
            if mapping.len() != 1 {
                return Err(ParseError::Invalid(
                    "constraint maps must contain exactly one key".to_owned(),
                ));
            }

            let Some((key, constraint_value)) = mapping.iter().next() else {
                return Err(ParseError::Invalid("empty constraint map".to_owned()));
            };

            let key = key.as_str().ok_or_else(|| {
                ParseError::Invalid("constraint names must be strings".to_owned())
            })?;
            let text = scalar_to_string(constraint_value);

            Ok(match key {
                "signed_by" => Constraint::SignedBy(text),
                "before" => Constraint::Before(text),
                "after" => Constraint::After(text),
                "positive" => Constraint::Positive(text),
                "value_preserved" => {
                    let value = if matches!(constraint_value, Value::Null) {
                        None
                    } else {
                        Some(text)
                    };
                    Constraint::ValuePreserved(value)
                }
                "output_exists" => Constraint::OutputExists(text),
                "datum_field_equals" => parse_datum_field_equals(constraint_value)?,
                "custom_aiken" => Constraint::CustomAiken(text),
                other => Constraint::Unknown {
                    name: other.to_owned(),
                    value: if matches!(constraint_value, Value::Null) {
                        None
                    } else {
                        Some(text)
                    },
                },
            })
        }
        other => Err(ParseError::Invalid(format!(
            "constraint must be a string or single-key map, got `{}`",
            scalar_to_string(other)
        ))),
    }
}

fn parse_datum_field_equals(value: &Value) -> Result<Constraint, ParseError> {
    match value {
        Value::Mapping(mapping) => {
            let field_key = Value::String("field".to_owned());
            let value_key = Value::String("value".to_owned());
            let Some(field) = mapping.get(&field_key) else {
                return Err(ParseError::Invalid(
                    "`datum_field_equals` requires a `field` key".to_owned(),
                ));
            };
            let Some(expected) = mapping.get(&value_key) else {
                return Err(ParseError::Invalid(
                    "`datum_field_equals` requires a `value` key".to_owned(),
                ));
            };
            Ok(Constraint::DatumFieldEquals {
                field: scalar_to_string(field),
                value: scalar_to_string(expected),
            })
        }
        Value::String(text) => {
            let Some((field, expected)) = text.split_once('=') else {
                return Err(ParseError::Invalid(
                    "`datum_field_equals` string form must look like `field=value`".to_owned(),
                ));
            };
            Ok(Constraint::DatumFieldEquals {
                field: field.trim().to_owned(),
                value: expected.trim().to_owned(),
            })
        }
        other => Err(ParseError::Invalid(format!(
            "`datum_field_equals` must be a map or `field=value` string, got `{}`",
            scalar_to_string(other)
        ))),
    }
}

fn value_map_to_string_map(map: NamedMap<Value>) -> Result<BTreeMap<String, String>, ParseError> {
    Ok(map
        .into_entries()
        .into_iter()
        .map(|(key, value)| (key, scalar_to_string(&value)))
        .collect())
}

fn scalar_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(value) => value.clone(),
        Value::Sequence(_) | Value::Mapping(_) | Value::Tagged(_) => serde_yaml::to_string(value)
            .unwrap_or_default()
            .trim()
            .to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aikenflow_ir::Constraint;

    const SIMPLE_VAULT: &str = r#"
protocol: SimpleVault
assets:
  - name: ada
    kind: lovelace
states:
  Locked:
    datum:
      owner: PubKeyHash
      amount: Lovelace
      deadline: POSIXTime
transitions:
  Deposit:
    inputs: []
    outputs:
      - state: Locked
        value:
          ada: "$amount"
    constraints:
      - signed_by: "$owner"
      - positive: "$amount"
  Withdraw:
    inputs:
      - state: Locked
    outputs: []
    constraints:
      - signed_by: "datum.owner"
      - after: "datum.deadline"
invariants:
  - name: withdrawal_requires_owner
    expression: "transition.Withdraw requires signature(datum.owner)"
"#;

    #[test]
    fn parses_document_example_shape() {
        let protocol = parse_protocol(SIMPLE_VAULT).expect("valid protocol");

        assert_eq!(protocol.name, "SimpleVault");
        assert_eq!(protocol.states[0].name, "Locked");
        assert_eq!(protocol.transitions.len(), 2);
        assert!(matches!(
            protocol.transitions[1].constraints[0],
            Constraint::SignedBy(_)
        ));
    }

    #[test]
    fn rejects_unknown_top_level_fields() {
        let error = parse_protocol(
            r#"
protocol: Broken
unexpected: true
states: {}
"#,
        )
        .expect_err("unknown fields should be rejected");

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn rejects_duplicate_state_names_before_ir() {
        let error = parse_protocol(
            r#"
protocol: Broken
states:
  Locked:
    datum: {}
  Locked:
    datum: {}
"#,
        )
        .expect_err("duplicate state names should be rejected");

        assert!(error.to_string().contains("duplicate declaration `Locked`"));
    }

    #[test]
    fn rejects_duplicate_transition_names_before_ir() {
        let error = parse_protocol(
            r#"
protocol: Broken
states:
  Locked:
    datum: {}
transitions:
  Spend:
    inputs: []
  Spend:
    inputs: []
"#,
        )
        .expect_err("duplicate transition names should be rejected");

        assert!(error.to_string().contains("duplicate declaration `Spend`"));
    }

    #[test]
    fn rejects_duplicate_datum_field_names() {
        let error = parse_protocol(
            r#"
protocol: Broken
states:
  Locked:
    datum:
      owner: PubKeyHash
      owner: ByteArray
"#,
        )
        .expect_err("duplicate datum fields should be rejected");

        assert!(error.to_string().contains("duplicate declaration `owner`"));
    }

    #[test]
    fn rejects_unknown_transition_fields() {
        let error = parse_protocol(
            r#"
protocol: Broken
states:
  Locked:
    datum: {}
transitions:
  Spend:
    inputz: []
"#,
        )
        .expect_err("unknown transition fields should be rejected");

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn rejects_duplicate_output_value_keys() {
        let error = parse_protocol(
            r#"
protocol: Broken
states:
  Locked:
    datum: {}
transitions:
  Deposit:
    outputs:
      - state: Locked
        value:
          ada: 1
          ada: 2
"#,
        )
        .expect_err("duplicate output value keys should be rejected");

        assert!(error.to_string().contains("duplicate declaration `ada`"));
    }

    #[test]
    fn rejects_duplicate_output_datum_keys() {
        let error = parse_protocol(
            r#"
protocol: Broken
states:
  Locked:
    datum:
      owner: PubKeyHash
transitions:
  Deposit:
    outputs:
      - state: Locked
        datum:
          owner: "$owner"
          owner: "datum.owner"
"#,
        )
        .expect_err("duplicate output datum keys should be rejected");

        assert!(error.to_string().contains("duplicate declaration `owner`"));
    }
}

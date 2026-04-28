# Protocol Specification

The MVP specification format is YAML. YAML is not the product moat; the semantic IR is.

The parser is intentionally strict:

- Unknown fields are rejected instead of ignored.
- Duplicate YAML map keys are rejected for states, transitions, datum fields, output values, and output datum values.
- Declaration order is preserved for states, transitions, and datum fields so generated artefacts follow the spec.
- Parser errors are syntax/schema failures. Semantic issues are reported later as AikenFlow diagnostics from the IR.

## Top-Level Fields

- `protocol`: Required protocol name.
- `description`: Optional human-readable description.
- `assets`: Declared assets used by outputs, mints, and burns.
- `states`: Named state map.
- `transitions`: Named transition map.
- `invariants`: Human-readable invariant declarations.

The top-level field `name` is accepted as an alias for `protocol`.

## Assets

```yaml
assets:
  - name: ada
    kind: lovelace
  - name: regulated_token
    kind: native
    policy: regulated_policy
```

Asset fields:

- `name`: Stable protocol asset name. `outputs[].value`, `mints[].asset`, and `burns[].asset` must reference declared asset names.
- `kind`: Asset kind such as `lovelace` or `native`.
- `policy`: Optional policy identifier for native assets. Mint and burn effects must use the declared policy when one is present.

`lovelace` assets may appear in `outputs[].value`, but they cannot appear in `mints` or `burns`. ADA movement is represented by consuming and producing UTxOs, not minting.

## States

```yaml
states:
  Locked:
    datum:
      owner: PubKeyHash
      amount: Lovelace
      deadline: POSIXTime
```

Datum field types are mapped into generated Aiken and TypeScript as follows:

| Spec Type | Aiken | TypeScript |
|---|---|---|
| `PubKeyHash` | `VerificationKeyHash` | `string` |
| `Lovelace` | `Int` | `bigint` |
| `POSIXTime` | `Int` | `bigint` |
| `Int` | `Int` | `bigint` |
| `Bool` | `Bool` | `boolean` |
| `ByteArray` | `ByteArray` | `string` |
| unknown | `Data` | `unknown` |

State fields:

- `datum`: Optional map of datum field names to type names.
- `script`: Optional script label for future hook-based generation.
- `terminal`: Optional boolean. Terminal states may be produced but should not be consumed by later transitions.

## Transitions

```yaml
transitions:
  Withdraw:
    inputs:
      - state: Locked
    outputs: []
    constraints:
      - signed_by: "datum.owner"
      - after: "datum.deadline"
```

Supported constraint names:

- `signed_by`
- `before`
- `after`
- `positive`
- `value_preserved`
- `output_exists`
- `datum_field_equals`
- `custom_aiken`

Unknown constraints are accepted with warnings and preserved in reports. Generators do not enforce them yet.

Transition fields:

- `inputs`: Consumed states. Each item may be a state name or `{ state, alias }`.
- `reference_inputs`: Read-only states. Each item may be a state name or `{ state, alias }`.
- `alias`: Optional input binding name. Off-chain builders use it for parameter prefixes such as `sourceUtxos`, `sourceDatum`, and `sourceValidator`.
- `outputs`: Produced states with optional `value` and `datum` maps.
- `outputs[].value`: Optional asset assignments for the produced state. Off-chain builders emit explicit asset maps when present. If omitted on a transition that consumes state, builders carry assets from the consumed UTxOs. If omitted on a creation transition, builders emit an explicit runtime error instead of silently producing an empty asset map.
- `outputs[].datum`: Optional datum-field assignments for the produced state. Off-chain builders use these assignments as overrides on the generated output datum parameter.
- `mints`: Minted asset effects with `policy`, declared `asset`, and `amount`.
- `burns`: Burned asset effects with `policy`, declared `asset`, and `amount`.
- `constraints`: Ordered list of supported, custom, or unknown constraints.

## Diagnostics

The IR validator reports semantic issues after parsing, including:

- Empty protocol, asset, state, transition, datum field, constraint, or invariant values.
- Duplicate asset, state, transition, datum field, or invariant names.
- Protocol, state, transition, datum field, or input alias names that cannot generate valid identifiers, including names that normalize to Aiken or TypeScript reserved words.
- State, transition, or datum field names that collide after generated identifier normalization.
- Empty output value or output datum assignments.
- Creation outputs without explicit asset value assignments.
- Output, mint, or burn effects that reference undeclared assets.
- Mint or burn policy values that conflict with declared asset policies.
- Mint or burn effects that target `kind: lovelace` assets.
- Output datum assignments targeting fields not declared on the produced state.
- Empty input aliases or duplicate generated input bindings.
- Input aliases that collide with output state parameter prefixes.
- Duplicate output state parameter prefixes within one transition.
- Constraint type mismatches for generated checks, such as `signed_by` on non-key fields or `positive`/validity constraints on non-numeric fields.
- `datum_field_equals` constraints that target unresolved fields or compare incompatible field/literal types.
- Unknown input, reference input, or output states.
- Consuming terminal states.
- Unknown constraints that generators cannot enforce yet.
- Owner-like consumed datum fields without a signature constraint.

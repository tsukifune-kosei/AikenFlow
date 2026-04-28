use aikenflow_ir::{
    AssetDecl, Constraint, FieldDecl, InvariantDecl, Protocol, StateDecl, StateInput,
    TransitionDecl,
};

#[test]
fn simple_vault_aiken_output_matches_golden() {
    let files = aikenflow_aiken_gen::generate(&simple_vault_protocol());

    assert_eq!(
        files
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        vec![
            "aiken.toml",
            "README.md",
            "validators/simple_vault_locked.ak"
        ]
    );

    assert_file(
        &files,
        "aiken.toml",
        include_str!("golden/simple-vault-aiken.toml"),
    );
    assert_file(
        &files,
        "README.md",
        include_str!("golden/simple-vault-readme.md"),
    );
    assert_file(
        &files,
        "validators/simple_vault_locked.ak",
        include_str!("golden/simple-vault-validator.ak"),
    );
}

fn assert_file(files: &aikenflow_ir::GeneratedFiles, path: &str, expected: &str) {
    let actual = files
        .files
        .iter()
        .find(|file| file.path == path)
        .unwrap_or_else(|| panic!("missing generated file `{path}`"));

    assert_eq!(actual.content, expected);
}

fn simple_vault_protocol() -> Protocol {
    Protocol {
        name: "SimpleVault".to_owned(),
        description: Some(
            "Simple custody workflow with owner authorization and deadline-based withdrawal."
                .to_owned(),
        ),
        assets: vec![AssetDecl {
            name: "ada".to_owned(),
            kind: "lovelace".to_owned(),
            policy: None,
        }],
        states: vec![StateDecl {
            name: "Locked".to_owned(),
            datum: vec![
                FieldDecl {
                    name: "owner".to_owned(),
                    ty: "PubKeyHash".to_owned(),
                },
                FieldDecl {
                    name: "amount".to_owned(),
                    ty: "Lovelace".to_owned(),
                },
                FieldDecl {
                    name: "deadline".to_owned(),
                    ty: "POSIXTime".to_owned(),
                },
            ],
            script: None,
            terminal: false,
        }],
        transitions: vec![
            TransitionDecl {
                name: "Deposit".to_owned(),
                inputs: vec![],
                reference_inputs: vec![],
                outputs: vec![aikenflow_ir::StateOutput {
                    state: "Locked".to_owned(),
                    value: [("ada".to_owned(), "$amount".to_owned())].into(),
                    datum: Default::default(),
                }],
                mints: vec![],
                burns: vec![],
                constraints: vec![
                    Constraint::SignedBy("$owner".to_owned()),
                    Constraint::Positive("$amount".to_owned()),
                ],
                effects: vec![],
            },
            TransitionDecl {
                name: "Withdraw".to_owned(),
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
                    Constraint::After("datum.deadline".to_owned()),
                ],
                effects: vec![],
            },
            TransitionDecl {
                name: "EmergencyCancel".to_owned(),
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
                    Constraint::Before("datum.deadline".to_owned()),
                ],
                effects: vec![],
            },
        ],
        invariants: vec![
            InvariantDecl {
                name: "no_negative_locked_value".to_owned(),
                expression: "locked.amount >= 0".to_owned(),
            },
            InvariantDecl {
                name: "withdrawal_requires_owner".to_owned(),
                expression: "transition.Withdraw requires signature(datum.owner)".to_owned(),
            },
            InvariantDecl {
                name: "withdrawal_requires_deadline_passed".to_owned(),
                expression: "transition.Withdraw requires after(datum.deadline)".to_owned(),
            },
        ],
    }
}

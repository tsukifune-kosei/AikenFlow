#[test]
fn simple_vault_state_graph_matches_golden() {
    let protocol = aikenflow_parser::parse_protocol(include_str!(
        "../../../examples/simple-vault/protocol.yaml"
    ))
    .expect("simple vault fixture parses");
    let expected = include_str!("../../../tests/golden/simple-vault-state-graph.mmd");

    assert_eq!(
        aikenflow_assurance::generate_state_graph(&protocol),
        expected
    );
}

#[test]
fn simple_vault_assurance_output_matches_golden() {
    let protocol = aikenflow_parser::parse_protocol(include_str!(
        "../../../examples/simple-vault/protocol.yaml"
    ))
    .expect("simple vault fixture parses");
    let files = aikenflow_assurance::generate(&protocol);

    assert_eq!(
        files
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        vec![
            "README.md",
            "state-graph.mmd",
            "topology.json",
            "coverage.json",
            "invariant-matrix.md",
            "adversarial-cases.md",
            "tests/adversarial.spec.ts",
            "AUDIT.md",
        ]
    );

    assert_file(
        &files,
        "README.md",
        include_str!("golden/simple-vault-readme.md"),
    );
    assert_file(
        &files,
        "state-graph.mmd",
        include_str!("../../../tests/golden/simple-vault-state-graph.mmd"),
    );
    assert_file(
        &files,
        "topology.json",
        include_str!("golden/simple-vault-topology.json"),
    );
    assert_file(
        &files,
        "coverage.json",
        include_str!("golden/simple-vault-coverage.json"),
    );
    assert_file(
        &files,
        "invariant-matrix.md",
        include_str!("golden/simple-vault-invariant-matrix.md"),
    );
    assert_file(
        &files,
        "adversarial-cases.md",
        include_str!("golden/simple-vault-adversarial-cases.md"),
    );
    assert_file(
        &files,
        "tests/adversarial.spec.ts",
        include_str!("golden/simple-vault-adversarial.spec.ts"),
    );
    assert_file(
        &files,
        "AUDIT.md",
        include_str!("golden/simple-vault-audit.md"),
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

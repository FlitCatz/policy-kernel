#[test]
fn frozen_source_and_required_obligations_are_recorded() {
    let matrix_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/obligation-matrix.yaml");
    let matrix = std::fs::read_to_string(matrix_path).unwrap();
    assert!(matrix.contains("e5f6ec8584cc91fdeca3aab40687c6cdcd303b43"));
    for id in [
        "PK-PROV-001",
        "PK-SELF-001",
        "PK-SEV-001",
        "PK-WAIVE-001",
        "PK-SENSOR-001",
        "PK-ADEQ-001",
        "PK-PRE-001",
        "PK-POST-001",
    ] {
        assert!(matrix.contains(id), "missing obligation {id}");
    }
}

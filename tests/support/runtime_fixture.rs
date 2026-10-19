//! Built by the test harness before starting environment-isolated product CLIs.
fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--version") => println!("fixture-1"),
        Some("report") => {
            let millis = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis();
            let report = include_str!("../../crates/policy-report-adapters/tests/fixtures/project-reports/vitest.json")
                .replace("1756684800000", &millis.to_string());
            std::fs::create_dir_all("reports").unwrap();
            std::fs::write("reports/unit.json", report).unwrap();
            println!("fixture report written");
        }
        _ => std::process::exit(7),
    }
}

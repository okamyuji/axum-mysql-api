use std::process::Command;

#[test]
fn print_openapi_outputs_spec_without_database_or_api_key() {
    let output = Command::new(env!("CARGO_BIN_EXE_axum-mysql-api"))
        .arg("--print-openapi")
        .env_remove("DATABASE_URL")
        .env_remove("API_KEY")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let api: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(api["openapi"].as_str().unwrap().starts_with("3."));
    assert!(api["paths"]["/entries/{id}"]["get"].is_object());
    assert!(api["paths"]["/journals"]["post"].is_object());
    assert!(api["components"]["securitySchemes"]["bearerAuth"].is_object());
}

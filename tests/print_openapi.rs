use std::process::Command;

/// DBの無いCIでも仕様を出力できることを保証するため、環境変数を消して実バイナリを起動する。
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
    assert_eq!(api["openapi"], "3.1.0");
    assert!(api["paths"]["/entries/{id}"]["get"].is_object());
    assert!(api["paths"]["/journals"]["post"].is_object());
    assert!(api["components"]["securitySchemes"]["bearerAuth"].is_object());
}

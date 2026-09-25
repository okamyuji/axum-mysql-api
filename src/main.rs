mod auth;
mod domain;
mod handler;
mod repository;
mod service;

use axum::{middleware, Router};
use repository::EntryRepository;
use service::EntryService;
use sqlx::mysql::MySqlPoolOptions;
use utoipa::openapi::{
    security::{Http, HttpAuthScheme, SecurityScheme},
    OpenApi,
};
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

/// 返す `Router` には認証が掛かっていない。サーバーに組み込むときは、
/// 必ず `auth::require_api_key` を `route_layer` として掛けること。
fn api_router() -> (Router<EntryService<EntryRepository>>, OpenApi) {
    let (router, mut api) = OpenApiRouter::new()
        .routes(routes!(handler::get_entry, handler::create_journal))
        .split_for_parts();
    api.components
        .get_or_insert_with(Default::default)
        .add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    (router, api)
}

/// 認証はAPIルートにだけ掛かり、Swagger UIとOpenAPI仕様は認証なしで公開される。
fn app(service: EntryService<EntryRepository>, key_hash: [u8; 32]) -> Router {
    let (router, api) = api_router();
    router
        .route_layer(middleware::from_fn_with_state(
            key_hash,
            auth::require_api_key,
        ))
        .with_state(service)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
}

/// 第1引数が `--print-openapi` のときは、`DATABASE_URL` と `API_KEY` を読む前、DBに接続する前に仕様を出力して終了する。
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() == Some("--print-openapi") {
        println!("{}", api_router().1.to_pretty_json()?);
        return Ok(());
    }
    tracing_subscriber::fmt::init();
    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;
    let key = std::env::var("API_KEY")?;
    let key_hash = auth::key_hash(&key)?;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(
        listener,
        app(EntryService::new(EntryRepository::new(pool)), key_hash),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Entries;
    use testcontainers::{
        core::{IntoContainerPort, WaitFor},
        runners::AsyncRunner,
        ContainerAsync, GenericImage, ImageExt,
    };

    const KEY: &str = "a-long-random-test-api-key-1234567890";

    async fn database() -> (ContainerAsync<GenericImage>, sqlx::MySqlPool) {
        let container = GenericImage::new("mysql", "8.4")
            .with_exposed_port(3306.tcp())
            .with_wait_for(WaitFor::message_on_stderr("port: 3306"))
            .with_env_var("MYSQL_ROOT_PASSWORD", "test-password")
            .with_env_var("MYSQL_DATABASE", "accounting")
            .start()
            .await
            .expect("start MySQL");
        let port = container
            .get_host_port_ipv4(3306.tcp())
            .await
            .expect("MySQL port");
        let url = format!("mysql://root:test-password@127.0.0.1:{port}/accounting");
        let pool = MySqlPoolOptions::new()
            .connect(&url)
            .await
            .expect("connect MySQL");
        sqlx::raw_sql(include_str!("../schema.sql"))
            .execute(&pool)
            .await
            .expect("initialize schema");
        (container, pool)
    }

    #[test]
    fn api_key_minimum_length() {
        assert!(auth::key_hash(&"k".repeat(31)).is_err());
        assert!(auth::key_hash(&"k".repeat(32)).is_ok());
        assert!(auth::key_hash(&"k".repeat(33)).is_ok());
    }

    #[tokio::test]
    async fn accounting_api_end_to_end() {
        let (container, pool) = database().await;
        let repository = EntryRepository::new(pool.clone());
        let entry = repository.find_by_id(1).await.unwrap().unwrap();
        assert_eq!(
            (entry.account_code.as_str(), entry.amount_cents),
            ("1000", 12500)
        );

        let service = EntryService::new(repository);
        let sample = crate::domain::NewJournal {
            entries: vec![
                crate::domain::NewEntry {
                    account_code: "1000".into(),
                    amount_cents: 100,
                    description: "Valid".into(),
                },
                crate::domain::NewEntry {
                    account_code: "2000".into(),
                    amount_cents: -100,
                    description: "Valid".into(),
                },
            ],
        };
        let mut invalid = sample.clone();
        invalid.entries.pop();
        assert!(matches!(
            service.create_journal(invalid).await,
            Err(crate::service::CreateError::Invalid)
        ));
        let mut invalid = sample.clone();
        invalid.entries[0].account_code.clear();
        assert!(matches!(
            service.create_journal(invalid).await,
            Err(crate::service::CreateError::Invalid)
        ));
        let mut invalid = sample.clone();
        invalid.entries[0].account_code = "a".repeat(21);
        assert!(matches!(
            service.create_journal(invalid).await,
            Err(crate::service::CreateError::Invalid)
        ));
        let mut invalid = sample.clone();
        invalid.entries[0].description = "a".repeat(256);
        assert!(matches!(
            service.create_journal(invalid).await,
            Err(crate::service::CreateError::Invalid)
        ));
        let mut invalid = sample.clone();
        invalid.entries[0].amount_cents = 0;
        assert!(matches!(
            service.create_journal(invalid).await,
            Err(crate::service::CreateError::Invalid)
        ));

        let mut too_many = sample.clone();
        too_many.entries.resize(101, too_many.entries[0].clone());
        assert!(matches!(
            service.create_journal(too_many).await,
            Err(crate::service::CreateError::Invalid)
        ));

        let mut valid_boundary = sample;
        valid_boundary.entries[0].account_code = "a".repeat(20);
        valid_boundary.entries[0].description = "a".repeat(255);
        assert_eq!(
            service
                .create_journal(valid_boundary)
                .await
                .unwrap()
                .entry_ids
                .len(),
            2
        );

        let three_entries = crate::domain::NewJournal {
            entries: vec![
                crate::domain::NewEntry {
                    account_code: "1000".into(),
                    amount_cents: 50,
                    description: "Split".into(),
                },
                crate::domain::NewEntry {
                    account_code: "2000".into(),
                    amount_cents: 50,
                    description: "Split".into(),
                },
                crate::domain::NewEntry {
                    account_code: "3000".into(),
                    amount_cents: -100,
                    description: "Split".into(),
                },
            ],
        };
        assert_eq!(
            service
                .create_journal(three_entries)
                .await
                .unwrap()
                .entry_ids
                .len(),
            3
        );

        let router = app(service, auth::key_hash(KEY).unwrap());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = reqwest::Client::new();
        let base = format!("http://{address}");

        assert_eq!(
            client
                .get(format!("{base}/entries/1"))
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        assert_eq!(
            client
                .get(format!("{base}/entries/1"))
                .bearer_auth("wrong-key")
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        let entry = client
            .get(format!("{base}/entries/2"))
            .bearer_auth(KEY)
            .send()
            .await
            .unwrap();
        assert_eq!(entry.status(), 200);
        let body: serde_json::Value = entry.json().await.unwrap();
        assert_eq!(body["amount_cents"], -12500);
        assert_eq!(
            client
                .get(format!("{base}/entries/999"))
                .bearer_auth(KEY)
                .send()
                .await
                .unwrap()
                .status(),
            404
        );

        let journal = serde_json::json!({"entries": [
            {"account_code": "1000", "amount_cents": -2500, "description": "Payment"},
            {"account_code": "2000", "amount_cents": 2500, "description": "Payable"}
        ]});
        assert_eq!(
            client
                .post(format!("{base}/journals"))
                .json(&journal)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        let created = client
            .post(format!("{base}/journals"))
            .bearer_auth(KEY)
            .json(&journal)
            .send()
            .await
            .unwrap();
        assert_eq!(created.status(), 201);
        let body: serde_json::Value = created.json().await.unwrap();
        let id = body["id"].as_i64().unwrap();
        let entry_id = body["entry_ids"][0].as_i64().unwrap();
        let saved: serde_json::Value = client
            .get(format!("{base}/entries/{entry_id}"))
            .bearer_auth(KEY)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(
            (saved["journal_id"].as_i64(), saved["amount_cents"].as_i64()),
            (Some(id), Some(-2500))
        );
        let balance: (i64, i64) = sqlx::query_as(
            "SELECT COUNT(*), CAST(SUM(amount_cents) AS SIGNED) FROM journal_entries WHERE journal_id = ?",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(balance, (2, 0));

        let invalid = serde_json::json!({"entries": [
            {"account_code": "1000", "amount_cents": 100, "description": "Invalid"},
            {"account_code": "2000", "amount_cents": -99, "description": "Invalid"}
        ]});
        assert_eq!(
            client
                .post(format!("{base}/journals"))
                .bearer_auth(KEY)
                .json(&invalid)
                .send()
                .await
                .unwrap()
                .status(),
            422
        );
        let journal_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM journals")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(journal_count.0, 4);

        sqlx::raw_sql("CREATE TRIGGER reject_test_entry BEFORE INSERT ON journal_entries FOR EACH ROW SET NEW.account_code = IF(NEW.account_code = 'FAIL', NULL, NEW.account_code)")
            .execute(&pool).await.unwrap();
        let db_failure = serde_json::json!({"entries": [
            {"account_code": "1000", "amount_cents": -100, "description": "Rollback"},
            {"account_code": "FAIL", "amount_cents": 100, "description": "Rollback"}
        ]});
        assert_eq!(
            client
                .post(format!("{base}/journals"))
                .bearer_auth(KEY)
                .json(&db_failure)
                .send()
                .await
                .unwrap()
                .status(),
            500
        );
        let journal_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM journals")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(journal_count.0, 4);

        let api: serde_json::Value = client
            .get(format!("{base}/api-docs/openapi.json"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert!(api["paths"]["/journals"]["post"].is_object());
        assert!(api["components"]["securitySchemes"]["bearerAuth"].is_object());
        assert!(client
            .get(format!("{base}/swagger-ui/"))
            .send()
            .await
            .unwrap()
            .status()
            .is_success());

        sqlx::query("CREATE DATABASE accounting_legacy")
            .execute(&pool)
            .await
            .unwrap();
        let port = container.get_host_port_ipv4(3306.tcp()).await.unwrap();
        let legacy_url = format!("mysql://root:test-password@127.0.0.1:{port}/accounting_legacy");
        let legacy = MySqlPoolOptions::new().connect(&legacy_url).await.unwrap();
        sqlx::raw_sql("CREATE TABLE journal_entries (id BIGINT PRIMARY KEY, account_code VARCHAR(20) NOT NULL, amount_cents BIGINT NOT NULL, description VARCHAR(255) NOT NULL); INSERT INTO journal_entries VALUES (1, '1000', 12500, 'Cash receipt'), (2, '4000', -12500, 'Sales revenue');")
            .execute(&legacy).await.unwrap();
        let migrate = |database: &str| {
            std::process::Command::new("sh")
                .arg(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/scripts/migrate-legacy.sh"
                ))
                .env("MYSQL_DATABASE", database)
                .env("MYSQL_USER", "root")
                .env("MYSQL_PWD", "test-password")
                .env("MYSQL_PORT", port.to_string())
                .output()
                .expect("mysql client required for migration test")
        };
        sqlx::query("INSERT INTO journal_entries VALUES (3, '9999', 1, 'Unknown')")
            .execute(&legacy)
            .await
            .unwrap();
        assert!(!migrate("accounting_legacy").status.success());
        let untouched: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM journal_entries")
            .fetch_one(&legacy)
            .await
            .unwrap();
        assert_eq!(untouched.0, 3);
        sqlx::query("DELETE FROM journal_entries WHERE id = 3")
            .execute(&legacy)
            .await
            .unwrap();
        let result = migrate("accounting_legacy");
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(migrate("accounting_legacy").status.success());
        assert!(migrate("accounting").status.success());
        let migrated: (i64, i64) = sqlx::query_as("SELECT COUNT(*), CAST(SUM(amount_cents) AS SIGNED) FROM journal_entries WHERE journal_id = 1")
            .fetch_one(&legacy).await.unwrap();
        assert_eq!(migrated, (2, 0));
        assert!(sqlx::query("INSERT INTO journal_entries (journal_id, account_code, amount_cents, description) VALUES (999, '1000', 1, 'Invalid')")
            .execute(&legacy).await.is_err());
        legacy.close().await;

        server.abort();
        pool.close().await;
        container.stop().await.unwrap();
    }
}

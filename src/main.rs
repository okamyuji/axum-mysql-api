mod handler;
mod repository;
mod service;

use axum::Router;
use repository::EntryRepository;
use service::EntryService;
use sqlx::mysql::MySqlPoolOptions;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

fn app(service: EntryService<EntryRepository>) -> Router {
    let (router, api) = OpenApiRouter::new()
        .routes(routes!(handler::get_entry))
        .with_state(service)
        .split_for_parts();
    router.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app(EntryService::new(EntryRepository::new(pool)))).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::Entries;
    use testcontainers::{
        core::{IntoContainerPort, WaitFor},
        runners::AsyncRunner,
        ContainerAsync, GenericImage, ImageExt,
    };
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

    #[tokio::test]
    async fn http_and_documentation_work_end_to_end() {
        let (container, pool) = database().await;
        let repository = EntryRepository::new(pool.clone());
        let entry = repository.find_by_id(1).await.unwrap().unwrap();
        assert_eq!(
            (entry.account_code.as_str(), entry.amount_cents),
            ("1000", 12500)
        );

        let _ = tracing_subscriber::fmt().with_test_writer().try_init();

        let router = app(EntryService::new(EntryRepository::new(pool.clone())));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = reqwest::Client::new();
        let base = format!("http://{address}");

        let entry = client
            .get(format!("{base}/entries/2"))
            .send()
            .await
            .unwrap();
        assert_eq!(entry.status(), 200);
        let body: serde_json::Value = entry.json().await.unwrap();
        assert_eq!(body["amount_cents"], -12500);
        assert_eq!(
            client
                .get(format!("{base}/entries/999"))
                .send()
                .await
                .unwrap()
                .status(),
            404
        );
        let api: serde_json::Value = client
            .get(format!("{base}/api-docs/openapi.json"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert!(api["paths"]["/entries/{id}"]["get"].is_object());
        assert!(client
            .get(format!("{base}/swagger-ui/"))
            .send()
            .await
            .unwrap()
            .status()
            .is_success());
        server.abort();
        pool.close().await;
        container.stop().await.unwrap();
    }
}

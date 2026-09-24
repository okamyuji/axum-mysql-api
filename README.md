# axum-mysql-api

会計仕訳行を取得する最小の3層APIです。金額は整数の最小通貨単位で保持します。認証、仕訳の貸借一致、書き込みAPIは対象外です。

## 起動

```sh
docker compose up -d mysql
DATABASE_URL=mysql://accounting:accounting@127.0.0.1:3306/accounting cargo run
```

- `GET /entries/1`: 仕訳行を取得（見つからない場合404）
- `/swagger-ui/`: API仕様のUI
- `/api-docs/openapi.json`: OpenAPI仕様

`schema.sql` は新規MySQLコンテナ起動時に適用されます。Docker Composeのパスワードは開発専用です。本番では外部から与える `DATABASE_URL` と管理済みのスキーマ移行を使用してください。

## 検証

Dockerデーモンが必要です。Testcontainersは1回のE2Eテスト内でMySQLを起動し、異なる固定IDのSQL検証とHTTP検証で共有して終了時に停止します。データ更新はありません。HTTPの主要導線（200、404、OpenAPI JSON、Swagger UI）を実際のTCPサーバーとMySQLで確認します。Colimaの場合は `DOCKER_HOST=unix://$HOME/.colima/default/docker.sock` を設定します。

```sh
cargo test
cargo llvm-cov --summary-only
cargo mutants --timeout 120
```

# axum-mysql-api

axum / SQLx / utoipaによる会計仕訳APIです。`repository.rs` がMySQL、`service.rs` が貸借一致の検証、`handler.rs` がHTTPとOpenAPIを担当します。金額は整数の最小通貨単位で、正数と負数の合計がゼロになる仕訳を保存します。

## 起動

```sh
docker compose up -d mysql
export API_KEY='32バイト以上のランダムな秘密値'
DATABASE_URL=mysql://accounting:accounting@127.0.0.1:3306/accounting cargo run
```

APIルートは `Authorization: Bearer <API_KEY>` を要求します。

- `GET /entries/{id}`: 仕訳行を取得（見つからない場合404）
- `POST /journals`: 貸借一致する2行以上の仕訳をトランザクションで保存（201）、不正入力は422
- `/swagger-ui/`: Bearer認証を設定できるSwagger UI
- `/api-docs/openapi.json`: OpenAPI仕様

```sh
curl -H "Authorization: Bearer $API_KEY" -H 'Content-Type: application/json' \
  -d '{"entries":[{"account_code":"1000","amount_cents":-2500,"description":"Payment"},{"account_code":"2000","amount_cents":2500,"description":"Payable"}]}' \
  http://127.0.0.1:3000/journals
```

`schema.sql` は新規MySQLコンテナ起動時に適用されます。旧テンプレートで作成したDBは、アプリを止めてバックアップを取得した後、MySQLクライアントで手動移行してください。

```sh
MYSQL_DATABASE=accounting MYSQL_USER=accounting MYSQL_PWD=accounting ./scripts/migrate-legacy.sh
```

スクリプトは既に移行済みなら変更せず終了します。旧テンプレートの初期データ2行以外が存在する場合や途中で失敗した状態は変更せず停止します（移行前の仕訳グループを推測できないため）。MySQLのDDLはトランザクションでロールバックできないため、途中失敗時はバックアップから復元し、手作業で状態を確認してください。テストにはDockerとホスト側の`mysql`コマンドが必要です。Docker Composeのパスワードは開発専用です。本番ではTLS終端の背後で起動し、秘密鍵を環境変数またはシークレット管理から与え、スキーマを移行ツールで適用してください。固定APIキーはサービス単位の認証であり、個人別権限・鍵ローテーション・監査証跡は対象外です。

## 検証

Dockerデーモンが必要です。Testcontainersは1回のE2Eテスト内でMySQLを起動し、読み取り・保存・集計・HTTP検証で共有して終了時に停止します。Colimaの場合は `DOCKER_HOST=unix://$HOME/.colima/default/docker.sock` を設定します。

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo llvm-cov --summary-only
cargo mutants --file src/handler.rs --file src/repository.rs --file src/service.rs --file src/auth.rs --exclude-re 'replace \+= with -=' --timeout 120
```

貸借の合計がゼロかだけを判定するため、`balance += amount` を `balance -= amount` に置き換える変異は数学的に同値です。mutation testではこの1件だけ除外し、コンパイル不能な変異は成立した変異の分母に含めません。

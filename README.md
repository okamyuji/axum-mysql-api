# axum-mysql-api

[axum](https://docs.rs/axum/0.8)、[SQLx](https://docs.rs/sqlx/0.8)、[utoipa](https://docs.rs/utoipa/5)で実装した複式簿記の仕訳APIです。借方と貸方の合計が一致する仕訳だけをMySQLに保存し、APIの仕様をOpenAPI 3.1として公開します。

## 特徴

- 仕訳は2〜100行の仕訳行で構成され、全行の金額の合計がゼロになる場合だけ保存されます。
- 金額は整数の最小通貨単位（`amount_cents`）で扱うため、浮動小数点の丸め誤差が生じません。
- 1件の仕訳はMySQLのトランザクション内で保存します。途中の行で失敗した場合は仕訳全体をロールバックします。
- APIルートは固定APIキーによるBearer認証で保護されています。キーはSHA-256ハッシュとして保持し、定数時間で比較します。
- OpenAPI仕様とSwagger UIをアプリ自身が配信します。

## 必要なもの

| ツール | 用途 |
|---|---|
| Rust（stable） | ビルドと実行 |
| Docker / Docker Compose | 開発用MySQL 8.4の起動と、テスト時のTestcontainers |
| `mysql` コマンド | 旧スキーマの移行スクリプトと、その移行を含むテスト |

## クイックスタート

開発用のMySQLを起動します。コンテナの初回起動時に `schema.sql` がテーブルとサンプル仕訳を作成します。

```sh
docker compose up -d mysql
```

32バイト以上のAPIキーを用意し、アプリを起動します。

```sh
export API_KEY="$(openssl rand -hex 32)"
export DATABASE_URL=mysql://accounting:accounting@127.0.0.1:3306/accounting
cargo run
```

アプリは `0.0.0.0:3000` で待ち受けます。サンプル仕訳の1行目を取得して、起動を確認してください。

```sh
curl -H "Authorization: Bearer $API_KEY" http://127.0.0.1:3000/entries/1
```

## 設定

アプリは環境変数から設定を読み込みます。どちらかが欠けている場合、アプリは起動せずに終了します。

| 変数 | 必須 | 内容 |
|---|---|---|
| `DATABASE_URL` | 必須 | MySQLの接続URLです。例は `mysql://user:pass@host:3306/accounting` です。 |
| `API_KEY` | 必須 | Bearer認証の秘密値です。32バイト未満の値を与えると起動に失敗します。 |

待ち受けアドレス（`0.0.0.0:3000`）とコネクションプールの上限（10接続）は `src/main.rs` に固定値で書かれています。

## API

### 認証

`/entries` と `/journals` へのリクエストには `Authorization: Bearer <API_KEY>` ヘッダーが必要です。ヘッダーが無い場合や値が一致しない場合、APIは本文なしで `401` を返します。Swagger UIとOpenAPI仕様のルートは認証なしで取得できます。

### エンドポイント

| メソッド | パス | 成功時 | 失敗時 |
|---|---|---|---|
| `GET` | `/entries/{id}` | `200` と仕訳行 | `401` / `404`（該当なし） / `500` |
| `POST` | `/journals` | `201` と作成した仕訳のID | `401` / `422`（検証エラー） / `500` |
| `GET` | `/swagger-ui/` | Swagger UI | なし |
| `GET` | `/api-docs/openapi.json` | OpenAPI仕様（JSON） | なし |

アプリ側で判定したエラーは、ステータスコードだけを返し、本文を持ちません。JSONとして読めない本文には `400`、`Content-Type: application/json` が無いリクエストには `415` を、axumのJSON抽出器が返します。

### 仕訳の検証ルール

`POST /journals` は、次の条件をすべて満たす仕訳だけを保存します。条件を1つでも破ると `422` になり、データベースには何も書き込みません。

- `entries` の行数は2以上100以下にします。
- `account_code` は1文字以上20文字以下にします。
- `description` は255文字以下にします。
- `amount_cents` は0以外の整数にします。借方を正、貸方を負で表します。
- 全行の `amount_cents` の合計は0にします。合計は `i128` で計算するため、`i64` の範囲で桁があふれることはありません。

### リクエスト例

仕訳を作成します。

```sh
curl -H "Authorization: Bearer $API_KEY" -H 'Content-Type: application/json' \
  -d '{"entries":[
        {"account_code":"1000","amount_cents":-2500,"description":"Payment"},
        {"account_code":"2000","amount_cents":2500,"description":"Payable"}
      ]}' \
  http://127.0.0.1:3000/journals
```

APIは作成した仕訳のIDと、各仕訳行のIDを入力順で返します。IDの値はデータベースの状態によって変わります。

```json
{"id":2,"entry_ids":[3,4]}
```

仕訳行を取得すると、所属する仕訳のIDも含めて返します。

```json
{"id":3,"journal_id":2,"account_code":"1000","amount_cents":-2500,"description":"Payment"}
```

## OpenAPI仕様の出力

OpenAPI仕様は、ハンドラーに付けた `#[utoipa::path]` 属性とスキーマ型の `ToSchema` 導出から生成されます。ファイルとして保存するには `--print-openapi` を付けて実行してください。

```sh
cargo run -- --print-openapi > openapi.json
```

このオプションを付けたアプリは、整形したJSONを標準出力に書き出して終了するだけです。MySQLへの接続も `DATABASE_URL` と `API_KEY` の設定も不要なので、CIで仕様を生成する用途にも向いています。

起動中のアプリからも、同じ内容の仕様を取得できます。

```sh
curl -s http://127.0.0.1:3000/api-docs/openapi.json -o openapi.json
```

Bearer認証は、仕様の中で `bearerAuth` というセキュリティスキームとして定義されています。出力したファイルは、[OpenAPI Generator](https://openapi-generator.tech/) などのクライアント生成ツールにそのまま渡せます。

仕様をブラウザで確認するには、`http://127.0.0.1:3000/swagger-ui/` を開いてください。画面右上の`Authorize`ボタンから `API_KEY` の値を入力すれば、Swagger UIで認証付きのリクエストを試せます。

## アーキテクチャ

ソースコードは責務ごとにモジュールを分けています。依存の向きは外側から `domain` へ向かい、`domain` は他のモジュールに依存しません。

```text
main.rs ──▶ handler.rs ──▶ service.rs ──▶ domain.rs ◀── repository.rs
   │            (HTTP)       (ユースケース)   (モデルと規則)     (MySQL)
   └──▶ auth.rs (Bearer認証ミドルウェア)
```

| ファイル | 層 | 役割 |
|---|---|---|
| `src/domain.rs` | ドメイン | 仕訳と仕訳行の型、貸借一致の検証規則、永続化の抽象である `Entries` トレイトを定義します。 |
| `src/service.rs` | アプリケーション | 検証とリポジトリ呼び出しを組み合わせ、ユースケースとして提供します。 |
| `src/repository.rs` | インフラストラクチャ | `Entries` をSQLxとMySQLで実装します。 |
| `src/handler.rs` | プレゼンテーション | HTTPリクエストをユースケースへ渡し、結果をステータスコードに変換します。OpenAPIの注釈もここに置きます。 |
| `src/auth.rs` | プレゼンテーション | Bearerトークンを検証するaxumミドルウェアです。 |
| `src/main.rs` | 起動 | 依存を組み立て、ルーターとOpenAPIを構成してサーバーを起動します。E2Eテストもこのファイルにあります。 |
| `tests/print_openapi.rs` | テスト | `--print-openapi` がデータベースなしで仕様を出力することを確認します。 |

`EntryService` は `Entries` トレイトに対してジェネリックです。そのため、サービスはMySQLの実装を直接知りません。

## データベーススキーマ

`schema.sql` は、仕訳を表す `journals` と、仕訳行を表す `journal_entries` の2テーブルを定義します。`journal_entries.journal_id` は外部キーで `journals.id` を参照するので、存在しない仕訳に行を追加できません。同じファイルには、貸借一致したサンプル仕訳1件（2行）の投入も含まれています。

`docker compose` は、データボリュームが空の初回起動時だけ `schema.sql` を適用します。スキーマを作り直したい場合は `docker compose down -v` でボリュームを削除してから起動してください。

### 旧スキーマからの移行

`journals` テーブルを持たない旧テンプレートのデータベースは、同梱のスクリプトで移行できます。アプリを停止し、バックアップを取得してから実行してください。

```sh
MYSQL_DATABASE=accounting MYSQL_USER=accounting MYSQL_PWD=accounting ./scripts/migrate-legacy.sh
```

接続先は `MYSQL_HOST`（既定値 `127.0.0.1`）と `MYSQL_PORT`（既定値 `3306`）で変更できます。スクリプトの動作は次のとおりです。

- 移行済みのデータベースに対しては、何も変更せずに正常終了します。
- 旧テンプレートの初期データ2行以外の行がある場合は、変更せずに停止します。移行前の仕訳グループを推測できないためです。
- 途中まで移行された状態を検出した場合も、変更せずに停止します。

MySQLのDDLはトランザクションでロールバックできません。移行が途中で失敗したときは、バックアップから復元し、状態を手作業で確認してください。

## 開発とテスト

テストはTestcontainersでMySQL 8.4のコンテナを起動し、リポジトリ、サービス、HTTP、OpenAPI、旧スキーマ移行までを1つのE2Eテストで検証します。実行にはDockerデーモンとホスト側の `mysql` コマンドが必要です。[Colima](https://github.com/abiosoft/colima) を使う場合は、先に `DOCKER_HOST` を設定します。

```sh
export DOCKER_HOST=unix://$HOME/.colima/default/docker.sock
```

変更を加えたら、次のコマンドで確認します。

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo llvm-cov --summary-only
cargo mutants --file src/domain.rs --file src/service.rs --file src/handler.rs \
  --file src/repository.rs --file src/auth.rs \
  --exclude-re 'replace \+= with -=' --timeout 120
```

カバレッジの計測には [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) を、mutation testには [cargo-mutants](https://mutants.rs/) を使います。貸借の検証は合計がゼロかどうかだけを判定するので、`balance += amount` を `balance -= amount` に置き換えた変異は元のコードと同じ結果になります。`--exclude-re` はこの1種類の変異だけを対象から外すための指定です。

## セキュリティ上の注意

- `docker-compose.yml` のパスワードは開発専用です。本番環境では使わないでください。
- 本番ではTLSを終端するリバースプロキシの背後でアプリを起動してください。アプリ自身はHTTPだけを話します。
- `API_KEY` とデータベースの認証情報は、環境変数またはシークレット管理サービスから与えてください。
- 本番のスキーマは、`schema.sql` の初回適用に頼らず、マイグレーションツールで管理してください。
- 固定APIキーは、サービス単位の認証を提供します。利用者ごとの権限、キーのローテーション、監査証跡はこのアプリの対象外です。

## ライセンス

[MIT License](LICENSE) で公開しています。

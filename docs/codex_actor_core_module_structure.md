# cellex-actor-core モジュール構成レビュー

## 目的
- `modules/actor-core/src` 配下の全ファイルを走査し、現状のレイヤと責務を俯瞰する。
- モジュール分割が散逸している現状課題を整理し、再編後の「あるべき姿」を提示する。
- 後続のリファクタリング着手時に参照できる指針と移行ステップをまとめる。

## 現状サマリ
トップレベルでは `api/`, `internal/`, `shared/`, `tests.rs`, `lib.rs` の 5 つに大別される。`rg --files modules/actor-core/src` および `find` により取得したディレクトリ一覧を整理すると以下の特徴がある。

| ドメイン | 主なパス | 主な内容 |
| --- | --- | --- |
| 公開 API | `api/actor`, `api/messaging`, `api/mailbox`, `api/supervision`, `api/extensions`, `api/actor_system`, `api/identity`, `api/failure_event_stream` | 型安全な API、DSL、エクステンション登録、メッセージング・メールボックス抽象、監視 API |
| ランタイム実装 | `internal/actor`, `internal/context`, `internal/guardian`, `internal/mailbox`, `internal/message`, `internal/scheduler`, `internal/actor_system`, `internal/supervision`, `internal/metrics`, `internal/runtime_state` | 実際のアクタープロセッサや guardian、スケジューラ、内部メールボックス、テレメトリ、テスト支援 |
| 共有部品 | `shared/failure_telemetry`, `shared/map_system.rs`, `shared/receive_timeout` | API と内部の両方から利用される共通実装 |
| スタブ・テスト | `tests.rs`, 各サブモジュール配下の `tests.rs` や `tests` ディレクトリ | `cfg(test)`、`feature="std"` 前提のユニットテスト |

`internal/scheduler` 直下では Ready Queue, Immediate, Receive Timeout といった挙動ごとに再帰的なサブモジュールが並び、`ready_queue_scheduler/` 配下だけで 8 ファイル以上が存在する。API 配下でも `actor/behavior`, `actor/context`, `mailbox`, `messaging`, `supervision` が深くネストし、ドメイン境界が把握しづらい状態となっている。

## 現状課題
- **レイヤ責務の曖昧化**: `shared/` にランタイム寄りの実装 (`receive_timeout`) とテレメトリ API が混在し、`internal` と `api` の境界を横断している。
- **スケジューラ関連の肥大化**: `internal/scheduler` 配下にビルダー、実装、テストサポート、ready queue の実装が横並びになり、用途別のまとまりが乏しい。
- **命名と配置の非対称性**: `api/actor` では `behavior/`, `context/`, `failure/` と役割別に整理されているのに対し、`internal/actor` では `actor_cell.rs`, `internal_props.rs` など単機能ファイルが散在している。
- **テスト支援コードが実装直下に混在**: `internal/mailbox/test_support/` など実装と同階層に強く依存するテストヘルパが置かれ、ビルド時に不要な公開が発生している。
- **ドキュメントとコード参照の乖離**: 現時点のモジュール構成をまとめた資料がなく、新規 contributor がエントリポイントを掴みにくい。

## あるべきモジュール構成案
API とランタイムを明確に分離し、共通部品は `runtime/shared` に集約する。推奨する構造イメージを以下に示す。

```text
actor-core/
  src/
    api/
      actor/
      messaging/
      mailbox/
      supervision/
      extensions/
      identity/
      failure_event_stream.rs
      prelude.rs (任意、再エクスポート禁止方針に沿って軽量化)
    runtime/
      actor/
      actor_system/
      scheduler/
        builder.rs
        ready_queue/
        receive_timeout/
        immediate/
      mailbox/
      supervision/
      telemetry/
      metrics/
      tests/
    shared/
      failure/
      receive_timeout/
      map_system.rs
    lib.rs
    tests.rs
```

### 再編ガイドライン
1. **レイヤ定義**: `internal/` を `runtime/` にリネームし、API からの直接 import を禁止する (必要な型は `shared/` へ移す)。
2. **サブドメインの再グルーピング**: `runtime/scheduler` 配下を `builder.rs`, `worker.rs`, `strategy/`, `tests/` のように階層化し、Ready Queue 固有コードは `ready_queue/` ディレクトリへ集約する。
3. **共有部品の整理**: `shared/receive_timeout` と `runtime/scheduler/receive_timeout` を整理し、境界や Trait 可視性 (`pub(crate)`) を調整する。
4. **テスト支援の隔離**: `runtime/tests/` 配下に `mailbox`, `scheduler` など単位でモジュールをまとめ、ビルド対象から外れるよう `cfg(test)` でガードする。
5. **ドキュメント整備**: 本ドキュメントをベースに `docs/design/actor-core/` 配下へ詳細設計資料を展開し、リファクタリングチケットを切る。

## 現状→あるべき姿のマッピング

| 現在のパス | 推奨移設先 | メモ |
| --- | --- | --- |
| `internal/actor/*` | `runtime/actor/*` | ファイル名は `cell.rs`, `props.rs` などドメイン名ベースに統一 |
| `internal/scheduler/ready_queue_scheduler/*` | `runtime/scheduler/ready_queue/*` | `ReadyQueueScheduler` 系をディレクトリへ集約し、worker 実装を `worker.rs` に統合 |
| `internal/scheduler/scheduler_builder.rs` | `runtime/scheduler/builder.rs` | テスト専用 `immediate()` は `cfg(test)` で別ファイル化 |
| `internal/scheduler/scheduler_spawn_context.rs` | `runtime/scheduler/context.rs` | フィールド名は現状維持、doc string を活用 |
| `internal/mailbox/test_support/*` | `runtime/tests/mailbox/*` | テストサポートは `cfg(test)` + `mod` で隔離 |
| `shared/receive_timeout/*` | `runtime/shared/receive_timeout/*` | Trait 可視性 (`pub(crate)`) 問題を解消しつつ配置見直し |
| `shared/failure_telemetry/*` | `runtime/telemetry/*` | API との境界を明確化し、`api/supervision/telemetry` は純粋なインターフェース層に限定 |

## リファクタリング優先度
1. **スケジューラ領域の整理** (最もファイル数が多く、テレメトリ・ガーディアン系と密接に結合)。
2. **共有部品の責務再分離** (`receive_timeout`, `failure_telemetry`).
3. **テスト支援モジュールの隔離** (`internal/mailbox/test_support` など)。
4. **API 層からの不要な内部 import の排除** (`use crate::internal::*` を避ける)。

## 実施ステップ例
1. `internal/` を `runtime/` にリネームし、`lib.rs` と既存モジュールパスを更新。
2. `runtime/scheduler` 配下を提案構造に従い移動し、`ReadyQueueScheduler` と関連テストを切り出す。
3. `shared/receive_timeout` を `runtime/shared/receive_timeout` へ移し、公開境界を再設定。
4. `shared/failure_telemetry` を `runtime/telemetry` へ移設し、API 側は trait と DTO のみに縮退。
5. `docs/design/actor-core/` にリファクタリングログを追加し、CI で `cargo doc` を活用して構造変更の回 regresion を監視。

## 付録: 現状ディレクトリ一覧
主要ディレクトリを 2 階層まで抜粋したリスト。詳細なファイルリストは `rg --files modules/actor-core/src` の出力を参照。

```text
modules/actor-core/src
├── api
│   ├── actor
│   ├── actor_runtime
│   ├── actor_system
│   ├── extensions
│   ├── failure_event_stream
│   ├── identity
│   ├── mailbox
│   ├── messaging
│   └── supervision
├── internal
│   ├── actor
│   ├── actor_system
│   ├── context
│   ├── guardian
│   ├── mailbox
│   ├── message
│   ├── metrics
│   ├── scheduler
│   └── supervision
├── shared
│   ├── failure_telemetry
│   └── receive_timeout
├── api.rs
├── internal.rs
├── lib.rs
└── shared.rs
```

このドキュメントをベースに、今後のモジュール整理や設計レビューを進めてください。

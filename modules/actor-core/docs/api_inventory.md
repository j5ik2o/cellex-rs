# actor-core API/SPI 棚卸し（MECE分類・2025-10-08）

本ドキュメントは actor-core の公開 API と内部 SPI を MECE（Mutually Exclusive, Collectively Exhaustive）に分類し、再編タスクの基準とする。

## API レイヤ（ユーザー向け公開インターフェイス）

| カテゴリ | シンボル | 実配置 | 備考 |
| --- | --- | --- | --- |
| actor | `ActorRef`, `ActorSystem`, `Props`, `Behavior`, `Behaviors`, `SupervisorStrategy`, `MessageAdapterRef`, `TypedContext`(旧`Context`), `RootContext` | `modules/actor-core/src/api/actor/*.rs` | Typed DSL（`receive`/`supervise`/`message_adapter`）と基本操作レイヤ（TypedContext は後日リネーム予定） |
| messaging | `MessageEnvelope` | `modules/actor-core/src/api/messaging/message_envelope.rs` | ユーザーメッセージとシステムメッセージの橋渡し |
| identity | `ActorId`, `ActorPath` | `modules/actor-core/src/api/identity/{actor_id.rs,actor_path.rs}` | ルーティング／名前解決用 ID 型 |
| system-support | `Mailbox`, `MailboxFactory`, `MailboxSignal`, `PriorityEnvelope`, `SystemMessage`, `Spawn`, `Timer` | `modules/actor-core/src/api/actor/system_support.rs`（実体は `runtime/mailbox/*` など） | std/embedded 両対応の抽象境界。`ActorSystem` 初期化は共通ファクトリ経由に一本化 |
| supervision | `Supervisor`, `SupervisorDirective`, `NoopSupervisor`, `FailureEvent`, `FailureEscalationStage`, `EscalationSink`, `FailureEventHandler`, `FailureEventListener`, `RootEscalationSink` | `modules/actor-core/src/api/supervision/*.rs` | ユーザー拡張ポイントとして公開する監督/失敗ハンドラ |
| shared | `Shared`, `StateCell` | 外部クレート (`cellex_utils_core_rs`) を `api/shared.rs` で再エクスポート | 共有状態抽象 |
| event_stream | `FailureEventStream` | `modules/actor-core/src/api/event_stream.rs` | 実装は `actor-std` / `actor-embedded` など外部クレート側で提供 |

## Runtime レイヤ（内部実装・pub(crate)）

| カテゴリ | シンボル | 実配置 | 備考 |
| --- | --- | --- | --- |
| context | `ActorContext`, `ChildSpawnSpec`, `InternalActorRef` | `modules/actor-core/src/internal/context/{actor_context.rs,child_spawn_spec.rs,internal_actor_ref.rs}` | API 側では `crate::internal::context` 経由で参照 |
| system | `InternalActorSystem`, `InternalRootContext`, `InternalProps` | `modules/actor-core/src/internal/system/{internal_actor_system.rs,internal_root_context.rs,internal_props.rs}` | スケジューラ／ガーディアン連携の中核 |
| mailbox | `PriorityEnvelope`, `QueueMailbox*`, `MailboxOptions`, `SystemMessage` | `modules/actor-core/src/internal/mailbox/{messages.rs,queue_mailbox.rs,traits.rs}` | API からは `api::actor::system_support` を介して公開可否を制御 |
| scheduler | `ReadyQueueScheduler`, `ActorCell` | `modules/actor-core/src/internal/scheduler/{ready_queue_scheduler.rs,actor_cell.rs}` | ReadyQueue ベースのスケジューラ本体（外部には未公開） |
| guardian | `Guardian`, `GuardianStrategy` 実装, `ChildRecord` | `modules/actor-core/src/internal/guardian/{guardian.rs,strategy.rs,child_record.rs}` | API には戦略インターフェイスのみ再公開予定 |
| supervision | `CompositeEscalationSink`, `CustomEscalationSink`, `ParentGuardianSink`, `RootEscalationSink` 等 | `modules/actor-core/src/internal/supervision/{parent_guardian_sink.rs,root_sink.rs,composite_sink.rs,custom_sink.rs,traits.rs}` | Root/Parent ガーディアン向け内部シンク（API には trait/handler のみ公開） |

## Platform 層（Feature 切替境界）

| feature | シンボル | 実配置 | 備考 |
| --- | --- | --- | --- |
| `std` | JSON/Prost serializer 登録, ルートエスカレーション時の tracing ログ | `modules/actor-core/src/extensions.rs`, `modules/actor-core/src/api/supervision/escalation.rs` | 標準ライブラリ前提のシリアライザ・ログ機能を追加提供 |
| `alloc` | なし（共通化済み） | - | 現時点では共通コードで提供 |

## 公開可否ポリシー指針

- API レイヤは `pub`、Runtime/Platform は原則 `pub(crate)` で閉じる。
- Runtime の型を API が利用する場合は型 alias または専用 wrapper で公開する（例: `api::actor::system_support::MailboxFactory` は trait のみ公開し、具象型は内部）。
- `SystemMessage` は外部に晒さず、ユーザーは `MessageEnvelope::system()` のようなヘルパー経由で扱う設計に改める。

この分類を基準に、以降のステップでファイル配置と可視性を段階的に移行する。

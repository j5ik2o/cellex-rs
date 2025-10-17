# 理想的なモジュール構成

## 設計哲学

### 核心原則
1. **責務の明確な分離** - 各モジュールは単一の責務を持つ
2. **階層の浅さ** - 深いネストを避け、フラットな構造を優先
3. **命名の一貫性** - 予測可能で発見しやすい命名
4. **スケーラビリティ** - 新機能追加時に構造を壊さない

### アーキテクチャパターン
```
lib.rs (Public API re-exports)
  ↓
api/ (Public interfaces)
  ↓
internal/ (Implementation details)
  ↓
shared/ (Cross-cutting utilities)
```

## 推奨モジュール構成

### Root Level
```
src/
├── lib.rs              # Public API re-exports
├── api.rs              # API module aggregation
├── internal.rs         # Internal module aggregation
├── shared.rs           # Shared utilities aggregation
└── tests.rs            # Integration tests
```

### API層 (`api/`)

#### `api/actor/` - アクター
```
api/actor/
├── actor.rs            # Module root
├── actor_ref.rs        # ActorRef<T>
├── behavior.rs         # Behavior DSL (統合: behavior/*)
├── context.rs          # Context trait + implementations
├── props.rs            # Props builder
├── spawn.rs            # Spawn trait (from actor_system)
├── signal.rs           # Signal types
└── lifecycle/          # ライフサイクル関連
    ├── lifecycle.rs
    ├── shutdown.rs     # ShutdownToken
    └── root_context.rs # RootContext
```

**変更点**:
- `behavior/*` サブモジュールを `behavior.rs` に統合(Behaviorは単一ファイルで十分)
- `spawn` を `actor_system` から移動(アクター生成はアクターモジュールの責務)
- ライフサイクル関連を `lifecycle/` サブディレクトリに整理

#### `api/actor_system/` - アクターシステム
```
api/actor_system/
├── actor_system.rs     # Module root
├── system.rs           # ActorSystem type
├── config.rs           # Config + Builder統合
├── runner.rs           # ActorSystemRunner
└── timer.rs            # Timer abstraction
```

**変更点**:
- `builder` を `config.rs` に統合(Builderは設定の一部)
- `spawn` を削除(→ `api/actor/spawn.rs` へ移動)

#### `api/runtime/` - ランタイム
```
api/runtime/
├── runtime.rs          # Module root
├── actor_runtime.rs    # ActorRuntime trait
└── mailbox_runtime.rs  # MailboxRuntime trait (from api/mailbox)
```

**変更点**:
- `actor_runtime` から `runtime` にリネーム(より一般的)
- `MailboxRuntime` を `api/mailbox` から移動(ランタイム抽象化として統一)

#### `api/messaging/` - メッセージング(統合・簡略化)
```
api/messaging/
├── messaging.rs        # Module root
├── message.rs          # DynMessage + UserMessage統合
├── envelope.rs         # MessageEnvelope
├── sender.rs           # MessageSender
└── metadata/           # メタデータ統合
    ├── metadata.rs
    ├── metadata_storage.rs
    └── metadata_record.rs
```

**変更点**:
- `dyn_message` + `user_message` を `message.rs` に統合
- `dyn_message_value` を削除(内部実装詳細)
- `metadata_*` を `metadata/` サブディレクトリに整理

#### `api/mailbox/` - メールボックス(簡略化)
```
api/mailbox/
├── mailbox.rs          # Module root
├── mailbox_trait.rs    # Mailbox trait + MailboxPair
├── queue_mailbox.rs    # QueueMailbox実装(統合: queue_mailbox/*)
├── options.rs          # MailboxOptions
├── signal.rs           # MailboxSignal
├── concurrency.rs      # ThreadSafe + SingleThread
└── messages/           # システムメッセージ
    ├── messages.rs
    ├── system_message.rs
    ├── priority_envelope.rs
    └── priority_channel.rs
```

**変更点**:
- `mailbox_runtime` を削除(→ `api/runtime/` へ移動)
- `mailbox_producer` + `queue_mailbox_producer` を削除(実装詳細)
- `mailbox_handle` を削除(`mailbox_trait.rs` に統合)
- `queue_mailbox/*` を単一ファイルに統合

#### `api/supervision/` - スーパービジョン(統合)
```
api/supervision/
├── supervision.rs      # Module root
├── supervisor.rs       # Supervisor trait + NoopSupervisor
├── directive.rs        # SupervisorDirective
├── failure/            # 障害処理統合
│   ├── failure.rs
│   ├── failure_event.rs
│   ├── failure_info.rs
│   ├── failure_metadata.rs
│   └── escalation_stage.rs
├── escalation/         # エスカレーション
│   ├── escalation.rs
│   ├── escalation_sink.rs
│   ├── failure_handler.rs  # FailureEventHandler
│   └── failure_listener.rs # FailureEventListener
└── telemetry/          # テレメトリ
    ├── telemetry.rs
    ├── failure_telemetry.rs
    ├── failure_snapshot.rs
    ├── observation_config.rs
    └── implementations/
        ├── noop.rs
        └── tracing.rs
```

**変更点**:
- `api/actor/failure` を統合
- `api/failure_event_stream` を `escalation/` に統合
- `supervisor/*` を `supervisor.rs` に統合(小さいモジュール)
- テレメトリ実装を `implementations/` サブディレクトリに整理

#### `api/extensions/` - 拡張(変更なし)
```
api/extensions/
├── extensions.rs       # Module root
├── extension.rs        # Extension trait + ExtensionId
├── registry.rs         # Extensions registry
└── serializer.rs       # SerializerRegistryExtension
```

**変更点**: なし(既に適切な構造)

#### `api/identity/` - 削除提案
**理由**: 2ファイルのみで独立モジュールとして小さすぎる

**移動先**:
- `actor_id.rs` → `api/actor/actor_id.rs`
- `actor_path.rs` → `api/actor/actor_path.rs`

#### `api/ask/` - Ask パターン
```
api/ask/
├── ask.rs              # Module root
├── ask_future.rs       # AskFuture
├── ask_timeout.rs      # AskTimeoutFuture
└── ask_error.rs        # AskError
```

**変更点**:
- `api/actor/ask/*` から独立モジュールに昇格
- Ask パターンは重要な機能なので独立した方が発見しやすい

### Internal層 (`internal/`)

#### `internal/actor/` - アクター内部実装(変更なし)
```
internal/actor/
├── actor.rs            # Module root
├── actor_cell.rs       # ActorCell
└── internal_props.rs   # InternalProps
```

#### `internal/actor_system/` - システム内部実装(変更なし)
```
internal/actor_system/
├── actor_system.rs     # Module root
├── internal_actor_system.rs
├── internal_config.rs
└── internal_root_context.rs
```

#### `internal/context/` - コンテキスト実装(変更なし)
```
internal/context/
├── context.rs          # Module root
├── actor_context.rs
└── child_spawn_spec.rs
```

#### `internal/scheduler/` - スケジューラー(大幅簡略化)
```
internal/scheduler/
├── scheduler.rs        # Module root
├── core/               # コア抽象化
│   ├── core.rs
│   ├── actor_scheduler.rs
│   ├── scheduler_builder.rs
│   ├── spawn_context.rs
│   ├── spawn_error.rs
│   └── child_naming.rs
├── ready_queue/        # Ready Queueスケジューラー統合
│   ├── ready_queue.rs
│   ├── scheduler.rs    # ReadyQueueScheduler (統合: ready_queue_scheduler/*)
│   ├── worker.rs       # 統合: ready_queue_worker + ready_queue_worker_impl
│   ├── context.rs      # ReadyQueueContext
│   ├── state.rs        # ReadyQueueState
│   └── notifier.rs     # ReadyNotifier + ReadyEventHook
├── timeout/            # Receive Timeout統合
│   ├── timeout.rs
│   ├── driver.rs       # Driver trait + Noop実装
│   ├── scheduler.rs    # Scheduler trait + Noop実装
│   └── factory.rs      # Factory trait + Noop実装
└── test_support/
    └── immediate.rs    # ImmediateScheduler
```

**変更点**:
- **14サブモジュール → 3サブディレクトリ** に削減
- Noop系を通常実装と統合(feature flag or default impl)
- `receive_timeout` 関連を `timeout/` に統合
- `ready_queue_scheduler/*` (7ファイル)を5ファイルに統合

#### `internal/mailbox/` - メールボックス実装(簡略化)
```
internal/mailbox/
├── mailbox.rs          # Module root
├── priority_builder.rs # PriorityMailboxBuilder
├── spawner.rs          # Mailbox spawner
└── test_support/       # 変更なし
    ├── test_support.rs
    ├── test_mailbox_runtime.rs
    ├── test_signal.rs
    ├── test_signal_state.rs
    ├── test_signal_wait.rs
    ├── shared_backend_handle.rs
    └── common.rs
```

**変更点**: minor renaming

#### `internal/message/` - メッセージ実装(簡略化)
```
internal/message/
├── message.rs          # Module root
├── metadata.rs         # InternalMessageMetadata
├── sender.rs           # InternalMessageSender
└── metadata_table/     # メタデータテーブル統合
    ├── metadata_table.rs
    └── metadata_table_inner.rs
```

**変更点**: `metadata_table` 関連をサブディレクトリに整理

#### `internal/metrics/` - メトリクス(変更なし)
```
internal/metrics/
├── metrics.rs          # Module root
├── metrics_event.rs
├── metrics_sink.rs
├── metrics_sink_shared.rs
└── noop_metrics_sink.rs
```

#### `internal/guardian/` - ガーディアン(変更なし)
```
internal/guardian/
├── guardian.rs         # Module root
├── always_restart.rs
├── child_record.rs
└── guardian_strategy.rs
```

#### `internal/supervision/` - スーパービジョン実装(変更なし)
```
internal/supervision/
├── supervision.rs      # Module root
├── composite_escalation_sink.rs
├── custom_escalation_sink.rs
└── parent_guardian_sink.rs
```

#### `internal/runtime_state.rs` - ランタイム状態(変更なし)
単一ファイルモジュール、変更なし

### Shared層 (`shared/`)

#### `shared/failure_telemetry/` - テレメトリ共有型(簡略化)
```
shared/failure_telemetry/
├── failure_telemetry.rs    # Module root
├── telemetry.rs            # FailureTelemetry trait + Shared
├── handler.rs              # FailureEventHandler + Shared
├── listener.rs             # FailureEventListener + Shared
├── builder.rs              # Builder + BuilderFn + Context統合
└── telemetry_context.rs    # TelemetryContext (必要なら残す)
```

**変更点**:
- **6ファイル → 5ファイル** に削減
- `*_shared` サフィックスを削除(ディレクトリ名で明確)
- Builder関連を統合

#### `shared/timeout/` - Timeout共有型(簡略化)
```
shared/timeout/
├── timeout.rs          # Module root
├── driver.rs           # ReceiveTimeoutDriver + Bound + Shared統合
└── factory.rs          # ReceiveTimeoutFactory + Shared統合
```

**変更点**:
- **4ファイル → 2ファイル** に削減
- `receive_timeout` から `timeout` にリネーム(receive は冗長)
- `*_shared` サフィックス削除

#### `shared/map_system.rs` - Map System(削除提案)
**理由**: 単一ファイル、`MapSystemShared` 型のみ

**移動先**: `internal/message/map_system.rs` (メッセージング関連なので)

## モジュール数の変化

### Before (現在)
- **api**: 10モジュール、合計78ファイル
- **internal**: 10モジュール、合計66ファイル
- **shared**: 3モジュール、合計11ファイル
- **合計**: 23モジュール、155ファイル

### After (理想)
- **api**: 9モジュール(-1)、合計70ファイル(-8)
- **internal**: 10モジュール(±0)、合計60ファイル(-6)
- **shared**: 2モジュール(-1)、合計7ファイル(-4)
- **合計**: 21モジュール(-2)、137ファイル(-18)

**削減率**: ファイル数 11.6%削減、モジュール数 8.7%削減

## 移行戦略

### Phase 1: リスクの低い統合
1. `shared/*` の `*_shared` サフィックス削除
2. `api/identity` を `api/actor` に統合
3. `shared/map_system` を `internal/message` に移動

### Phase 2: Scheduler簡略化
1. Noop系を通常実装に統合
2. `receive_timeout` 関連を `timeout/` に統合
3. `ready_queue_scheduler/*` を5ファイルに統合

### Phase 3: メッセージング統合
1. `api/messaging` を6ファイルに削減
2. `api/mailbox` を9ファイルに削減
3. `internal/message` をサブディレクトリ化

### Phase 4: Supervision統合
1. `api/actor/failure` を `api/supervision/failure` に統合
2. `api/failure_event_stream` を `api/supervision/escalation` に統合

### Phase 5: 最終調整
1. `api/actor/ask` を独立モジュール `api/ask` に昇格
2. `api/spawn` を `api/actor/spawn` に移動
3. `api/runtime` モジュール作成

## 後方互換性

### `lib.rs` での Re-export維持
```rust
// 既存の公開型はすべて lib.rs で re-export を維持
pub use api::actor::actor_ref::ActorRef;
pub use api::actor::behavior::Behavior;
// ... 以下同様

// 削除された型のエイリアス(deprecated)
#[deprecated(since = "2.0.0", note = "Use `api::actor::ActorId` instead")]
pub use api::actor::actor_id::ActorId;
```

### 段階的移行パス
1. 新しいモジュールパスを追加
2. 古いパスに `#[deprecated]` を付与
3. 3-6ヶ月の deprecation 期間
4. 古いパスを削除

## まとめ

この理想的なモジュール構成は以下を実現します:

1. **18ファイル(11.6%)の削減** - 保守コスト削減
2. **より浅い階層** - 発見しやすさの向上
3. **明確な責務分離** - 新機能追加時の配置が明確
4. **一貫した命名** - 予測可能なモジュール構造
5. **後方互換性維持** - 段階的移行が可能

次のステップ: `migration-guide.md` で具体的な移行手順を文書化

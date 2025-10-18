# モジュール構成移行ガイド

## 移行の概要

### 目標
現在の155ファイル・23モジュール構成を137ファイル・21モジュール(-18ファイル, -11.6%)に削減し、より保守しやすい構造に移行します。

### 移行原則
1. **後方互換性維持**: `lib.rs` での re-export を維持し、既存コードを破壊しない
2. **段階的移行**: 5つのフェーズに分けてリスクを最小化
3. **テスト駆動**: 各フェーズ後にすべてのテストが通ることを確認
4. **Deprecation 期間**: 最低3ヶ月の猶予期間を設ける

## Phase 1: リスクの低い統合(優先度: High)

### 1.1 `shared/*` の `*_shared` サフィックス削除

#### 影響範囲
- `shared/failure_telemetry/*` (6ファイル)
- `shared/receive_timeout/*` (4ファイル)

#### 変更内容
```bash
# failure_telemetry
git mv shared/failure_telemetry/failure_event_handler_shared.rs \
       shared/failure_telemetry/handler.rs
git mv shared/failure_telemetry/failure_event_listener_shared.rs \
       shared/failure_telemetry/listener.rs
git mv shared/failure_telemetry/failure_telemetry_builder_shared.rs \
       shared/failure_telemetry/builder.rs
git mv shared/failure_telemetry/failure_telemetry_shared.rs \
       shared/failure_telemetry/telemetry.rs

# receive_timeout
git mv shared/receive_timeout/receive_timeout_driver_shared.rs \
       shared/receive_timeout/driver.rs
git mv shared/receive_timeout/receive_timeout_factory_shared.rs \
       shared/receive_timeout/factory.rs
```

#### `lib.rs` での後方互換対応
```rust
// Deprecated re-exports for backward compatibility
#[deprecated(since = "2.0.0", note = "Use `shared::failure_telemetry::handler::*` instead")]
pub use shared::failure_telemetry::handler as failure_event_handler_shared;

#[deprecated(since = "2.0.0", note = "Use `shared::failure_telemetry::listener::*` instead")]
pub use shared::failure_telemetry::listener as failure_event_listener_shared;

// ... 以下同様
```

#### 確認コマンド
```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
./scripts/ci.sh all
```

### 1.2 `api/identity` を `api/actor` に統合

#### 影響範囲
- `api/identity/actor_id.rs` → `api/actor/actor_id.rs`
- `api/identity/actor_path.rs` → `api/actor/actor_path.rs`
- `api/identity.rs` (削除)

#### 変更手順
```bash
# ファイル移動
git mv modules/actor-core/src/api/identity/actor_id.rs \
       modules/actor-core/src/api/actor/actor_id.rs
git mv modules/actor-core/src/api/identity/actor_path.rs \
       modules/actor-core/src/api/actor/actor_path.rs

# identity.rs 削除
git rm modules/actor-core/src/api/identity.rs
rm -rf modules/actor-core/src/api/identity/
```

#### `api/actor.rs` 修正
```rust
// api/actor.rs
pub mod actor_id;     // 追加
pub mod actor_path;   // 追加
pub mod actor_ref;
pub mod ask;
// ... existing modules
```

#### `lib.rs` での後方互換対応
```rust
// Maintain backward compatibility
pub use api::actor::actor_id;
pub use api::actor::actor_path;

// Deprecated module-level re-exports
#[deprecated(since = "2.0.0", note = "Use `api::actor::actor_id` instead")]
pub mod identity {
    pub use crate::api::actor::actor_id::*;
    pub use crate::api::actor::actor_path::*;
}
```

#### import 文の修正
```bash
# すべての `use crate::api::identity::` を置換
find modules/actor-core/src -name "*.rs" -exec sed -i '' \
  's/use crate::api::identity::/use crate::api::actor::/g' {} +
```

### 1.3 `shared/map_system` を `internal/message` に移動

#### 影響範囲
- `shared/map_system.rs` → `internal/message/map_system.rs`

#### 変更手順
```bash
git mv modules/actor-core/src/shared/map_system.rs \
       modules/actor-core/src/internal/message/map_system.rs
```

#### `internal/message.rs` 修正
```rust
// internal/message.rs
pub mod internal_message_metadata;
pub mod internal_message_sender;
pub mod map_system;  // 追加
pub mod metadata_table;
pub mod metadata_table_inner;
```

#### `lib.rs` での re-export 修正
```rust
// lib.rs
// Before
pub use shared::map_system::MapSystemShared;

// After
pub use internal::message::map_system::MapSystemShared;

// Deprecated
#[deprecated(since = "2.0.0", note = "Use `internal::message::map_system::MapSystemShared` instead")]
pub use internal::message::map_system::MapSystemShared as SharedMapSystem;
```

#### import 文の修正
```bash
find modules/actor-core/src -name "*.rs" -exec sed -i '' \
  's/use crate::shared::map_system::/use crate::internal::message::map_system::/g' {} +
```

### Phase 1 完了チェックリスト
- [ ] すべてのテストがパス: `cargo test --workspace`
- [ ] Clippy 警告なし: `cargo clippy --workspace -- -D warnings`
- [ ] CI スクリプト成功: `./scripts/ci.sh all`
- [ ] ドキュメント生成成功: `cargo doc --no-deps`
- [ ] git コミット: `git commit -m "refactor(actor-core): Phase 1 - low-risk consolidations"`

## Phase 2: Scheduler 簡略化(優先度: High)

### 2.1 Noop系の統合

#### 影響範囲
- `noop_receive_timeout_driver.rs`
- `noop_receive_timeout_scheduler.rs`
- `noop_receive_timeout_scheduler_factory.rs`

#### 戦略
Noop 実装を通常実装のデフォルト実装として統合:

```rust
// Before: 3つの独立ファイル
// noop_receive_timeout_scheduler_factory_provider
pub struct NoopReceiveTimeoutDriver;

// receive_timeout_driver.rs
pub trait ReceiveTimeoutDriver { ... }

// After: 1つのファイルに統合
// timeout/driver.rs
pub trait ReceiveTimeoutDriver { ... }

/// Default no-op implementation
#[derive(Debug, Clone, Default)]
pub struct NoopDriver;

impl ReceiveTimeoutDriver for NoopDriver { ... }
```

#### 変更手順
```bash
# 1. timeout/ ディレクトリ作成
mkdir -p modules/actor-core/src/internal/scheduler/timeout

# 2. receive_timeout 関連を統合
# driver.rs に統合
cat > modules/actor-core/src/internal/scheduler/timeout/driver.rs << 'EOF'
// ReceiveTimeoutDriver trait + Noop実装
EOF

# scheduler.rs に統合
cat > modules/actor-core/src/internal/scheduler/timeout/scheduler.rs << 'EOF'
// ReceiveTimeoutScheduler trait + Noop実装
EOF

# factory.rs に統合
cat > modules/actor-core/src/internal/scheduler/timeout/factory.rs << 'EOF'
// ReceiveTimeoutSchedulerFactory trait + Noop実装
EOF

# 3. 古いファイル削除
git rm modules/actor-core/src/internal/scheduler/noop_receive_timeout_*.rs
git rm modules/actor-core/src/internal/scheduler/receive_timeout_*.rs
git rm modules/actor-core/src/internal/scheduler/receive_timeout.rs
```

#### `internal/scheduler.rs` 修正
```rust
// Before
pub mod noop_receive_timeout_driver;
pub mod noop_receive_timeout_scheduler;
pub mod noop_receive_timeout_scheduler_factory;
pub mod receive_timeout;
pub mod receive_timeout_scheduler;
pub mod receive_timeout_scheduler_factory;

// After
pub mod timeout;  // 統合
```

#### `lib.rs` での re-export 修正
```rust
// Before
pub use internal::scheduler::NoopReceiveTimeoutDriver;
pub use internal::scheduler::ReceiveTimeoutScheduler;
pub use internal::scheduler::ReceiveTimeoutSchedulerFactory;

// After
pub use internal::scheduler::timeout::{
    NoopDriver as NoopReceiveTimeoutDriver,
    ReceiveTimeoutScheduler,
    ReceiveTimeoutSchedulerFactory,
};

// Deprecated
#[deprecated(since = "2.0.0", note = "Use `timeout::NoopDriver` instead")]
pub use internal::scheduler::timeout::NoopDriver as NoopReceiveTimeoutDriver;
```

### 2.2 `ready_queue_scheduler/*` の統合

#### 影響範囲(7ファイル → 5ファイル)
- `ready_queue_worker.rs` + `ready_queue_worker_impl.rs` → `worker.rs`
- `ready_notifier.rs` + `ready_event_hook.rs` → `notifier.rs`

#### 変更手順
```bash
# 1. worker 統合
cat modules/actor-core/src/internal/scheduler/ready_queue_scheduler/ready_queue_worker.rs \
    modules/actor-core/src/internal/scheduler/ready_queue_scheduler/ready_queue_worker_impl.rs \
    > modules/actor-core/src/internal/scheduler/ready_queue_scheduler/worker.rs

# 2. notifier 統合
cat modules/actor-core/src/internal/scheduler/ready_queue_scheduler/ready_notifier.rs \
    modules/actor-core/src/internal/scheduler/ready_queue_scheduler/ready_event_hook.rs \
    > modules/actor-core/src/internal/scheduler/ready_queue_scheduler/notifier.rs

# 3. リネーム
git mv modules/actor-core/src/internal/scheduler/ready_queue_scheduler/ready_queue_scheduler.rs \
       modules/actor-core/src/internal/scheduler/ready_queue_scheduler/scheduler.rs

# 4. 古いファイル削除
git rm modules/actor-core/src/internal/scheduler/ready_queue_scheduler/ready_queue_worker.rs
git rm modules/actor-core/src/internal/scheduler/ready_queue_scheduler/ready_queue_worker_impl.rs
git rm modules/actor-core/src/internal/scheduler/ready_queue_scheduler/ready_notifier.rs
git rm modules/actor-core/src/internal/scheduler/ready_queue_scheduler/ready_event_hook.rs
git rm modules/actor-core/src/internal/scheduler/ready_queue_scheduler/common.rs
```

#### `ready_queue_scheduler.rs` (モジュールroot) 修正
```rust
// Before
mod ready_queue_scheduler;
mod ready_queue_worker;
mod ready_queue_worker_impl;
mod ready_notifier;
mod ready_event_hook;
// ... others

pub use ready_queue_scheduler::*;
pub use ready_queue_worker::*;

// After
mod scheduler;
mod worker;
mod notifier;
mod context;
mod state;

pub use scheduler::*;
pub use worker::*;
pub use notifier::*;
```

### 2.3 Scheduler のディレクトリ構造整理

#### 最終的な構造
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
├── ready_queue/        # Ready Queue実装
│   ├── ready_queue.rs  # Module root
│   ├── scheduler.rs
│   ├── worker.rs       # 統合済み
│   ├── context.rs
│   ├── state.rs
│   └── notifier.rs     # 統合済み
├── timeout/            # Timeout機能
│   ├── timeout.rs      # Module root
│   ├── driver.rs       # 統合済み
│   ├── scheduler.rs    # 統合済み
│   └── factory.rs      # 統合済み
└── test_support/
    └── immediate.rs
```

#### 実装手順
```bash
# 1. core/ ディレクトリ作成と移動
mkdir -p modules/actor-core/src/internal/scheduler/core
git mv modules/actor-core/src/internal/scheduler/actor_scheduler.rs \
       modules/actor-core/src/internal/scheduler/core/actor_scheduler.rs
git mv modules/actor-core/src/internal/scheduler/scheduler_builder.rs \
       modules/actor-core/src/internal/scheduler/core/scheduler_builder.rs
git mv modules/actor-core/src/internal/scheduler/scheduler_spawn_context.rs \
       modules/actor-core/src/internal/scheduler/core/spawn_context.rs
git mv modules/actor-core/src/internal/scheduler/spawn_error.rs \
       modules/actor-core/src/internal/scheduler/core/spawn_error.rs
git mv modules/actor-core/src/internal/scheduler/child_naming.rs \
       modules/actor-core/src/internal/scheduler/core/child_naming.rs

# 2. core/core.rs 作成
cat > modules/actor-core/src/internal/scheduler/core/core.rs << 'EOF'
pub mod actor_scheduler;
pub mod scheduler_builder;
pub mod spawn_context;
pub mod spawn_error;
pub mod child_naming;

pub use actor_scheduler::*;
pub use scheduler_builder::*;
pub use spawn_context::*;
pub use spawn_error::*;
pub use child_naming::*;
EOF

# 3. scheduler.rs (module root) 更新
cat > modules/actor-core/src/internal/scheduler/scheduler.rs << 'EOF'
pub mod core;
pub mod ready_queue;
pub mod timeout;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

// Re-exports for backward compatibility
pub use core::*;
pub use ready_queue::*;
pub use timeout::*;
EOF
```

### Phase 2 完了チェックリスト
- [ ] すべてのテストがパス: `cargo test --workspace`
- [ ] import 文の修正: `grep -r "internal::scheduler::" src/ | grep -v "//"`
- [ ] CI スクリプト成功: `./scripts/ci.sh all`
- [ ] ベンチマーク動作確認: `cargo bench -p cellex-actor-core-rs`
- [ ] git コミット: `git commit -m "refactor(actor-core): Phase 2 - scheduler simplification"`

## Phase 3: メッセージング統合(優先度: Medium)

### 3.1 `api/messaging` の簡略化

#### 統合対象
- `dyn_message.rs` + `user_message.rs` → `message.rs`
- `dyn_message_value.rs` を削除(内部実装詳細として `message.rs` に統合)
- `metadata_*` 系を `metadata/` サブディレクトリに整理

#### 変更手順
```bash
# 1. metadata/ サブディレクトリ作成
mkdir -p modules/actor-core/src/api/messaging/metadata

# 2. metadata 関連の移動
git mv modules/actor-core/src/api/messaging/message_metadata.rs \
       modules/actor-core/src/api/messaging/metadata/metadata.rs
git mv modules/actor-core/src/api/messaging/metadata_storage.rs \
       modules/actor-core/src/api/messaging/metadata/storage.rs
git mv modules/actor-core/src/api/messaging/metadata_storage_mode.rs \
       modules/actor-core/src/api/messaging/metadata/storage_mode.rs
git mv modules/actor-core/src/api/messaging/metadata_storage_record.rs \
       modules/actor-core/src/api/messaging/metadata/storage_record.rs

# 3. message 統合
cat modules/actor-core/src/api/messaging/dyn_message.rs \
    modules/actor-core/src/api/messaging/user_message.rs \
    modules/actor-core/src/api/messaging/dyn_message_value.rs \
    > modules/actor-core/src/api/messaging/message.rs

# 4. 古いファイル削除
git rm modules/actor-core/src/api/messaging/dyn_message.rs
git rm modules/actor-core/src/api/messaging/user_message.rs
git rm modules/actor-core/src/api/messaging/dyn_message_value.rs
```

#### `api/messaging.rs` 修正
```rust
// Before
pub mod dyn_message;
pub mod dyn_message_value;
pub mod message_envelope;
pub mod message_metadata;
pub mod message_sender;
pub mod metadata_storage;
pub mod metadata_storage_mode;
pub mod metadata_storage_record;
pub mod user_message;

// After
pub mod message;          // 統合
pub mod envelope;         // renamed from message_envelope
pub mod sender;           // renamed from message_sender
pub mod metadata;         // submodule

pub use message::*;
pub use envelope::*;
pub use sender::*;
pub use metadata::*;
```

### 3.2 `api/mailbox` の簡略化

#### 統合対象
- `mailbox_producer.rs` + `queue_mailbox_producer.rs` を削除(内部実装詳細)
- `queue_mailbox/*.rs` (2ファイル)を `queue_mailbox.rs` に統合
- `mailbox_handle.rs` を `mailbox.rs` に統合

#### 変更手順
```bash
# 1. queue_mailbox 統合
cat modules/actor-core/src/api/mailbox/queue_mailbox/base.rs \
    modules/actor-core/src/api/mailbox/queue_mailbox/recv.rs \
    > modules/actor-core/src/api/mailbox/queue_mailbox_impl.rs

git rm -r modules/actor-core/src/api/mailbox/queue_mailbox/

# 2. mailbox_handle 統合
cat modules/actor-core/src/api/mailbox/mailbox_handle.rs \
    >> modules/actor-core/src/api/mailbox/mailbox.rs

git rm modules/actor-core/src/api/mailbox/mailbox_handle.rs

# 3. producer 削除
git rm modules/actor-core/src/api/mailbox/mailbox_producer.rs
git rm modules/actor-core/src/api/mailbox/queue_mailbox_producer.rs
```

#### `api/mailbox.rs` 修正
```rust
// Before
pub mod mailbox_concurrency;
pub mod mailbox_handle;
pub mod mailbox_options;
pub mod mailbox_producer;
pub mod mailbox_runtime;
pub mod mailbox_signal;
pub mod messages;
pub mod queue_mailbox;
pub mod queue_mailbox_producer;
pub mod single_thread;
pub mod thread_safe;

// After
pub mod mailbox;          // renamed, includes handle
pub mod mailbox_runtime;  // kept
pub mod options;          // renamed from mailbox_options
pub mod signal;           // renamed from mailbox_signal
pub mod concurrency;      // renamed from mailbox_concurrency
pub mod messages;         // kept as submodule
pub mod queue_mailbox;    // unified implementation

// Traits
pub use mailbox::*;
```

### Phase 3 完了チェックリスト
- [ ] すべてのテストがパス
- [ ] `api/messaging` が6ファイルに削減
- [ ] `api/mailbox` が9ファイルに削減
- [ ] import 文の自動修正スクリプト実行
- [ ] git コミット: `git commit -m "refactor(actor-core): Phase 3 - messaging consolidation"`

## Phase 4: Supervision 統合(優先度: Medium)

### 4.1 `api/actor/failure` を `api/supervision/failure` に統合

#### 影響範囲
- `api/actor/failure/*.rs` (3ファイル)
- `api/supervision/failure/*.rs` (既存5ファイル)

#### 変更手順
```bash
# 1. actor/failure の内容を supervision/failure に統合
# まず内容を確認
ls -la modules/actor-core/src/api/actor/failure/
ls -la modules/actor-core/src/api/supervision/failure/

# 2. 重複チェック後、統合
# (ActorFailure, BehaviorFailure, DefaultBehaviorFailure を supervision/failure に移動)

git mv modules/actor-core/src/api/actor/failure/actor_failure.rs \
       modules/actor-core/src/api/supervision/failure/actor_failure.rs
git mv modules/actor-core/src/api/actor/failure/behavior_failure.rs \
       modules/actor-core/src/api/supervision/failure/behavior_failure.rs
git mv modules/actor-core/src/api/actor/failure/default_behavior_failure.rs \
       modules/actor-core/src/api/supervision/failure/default_behavior_failure.rs

# 3. actor/failure/ ディレクトリ削除
git rm -r modules/actor-core/src/api/actor/failure/
```

#### `api/actor.rs` 修正
```rust
// Before
pub mod failure;

// After
// (削除 - supervision に統合)
```

#### `api/supervision/failure.rs` 修正
```rust
// 追加
pub mod actor_failure;
pub mod behavior_failure;
pub mod default_behavior_failure;

pub use actor_failure::*;
pub use behavior_failure::*;
pub use default_behavior_failure::*;
```

#### `lib.rs` での後方互換対応
```rust
// Deprecated re-exports
#[deprecated(since = "2.0.0", note = "Use `api::supervision::failure::ActorFailure` instead")]
pub use api::supervision::failure::ActorFailure;

#[deprecated(since = "2.0.0", note = "Use `api::supervision::failure::BehaviorFailure` instead")]
pub use api::supervision::failure::BehaviorFailure;
```

### 4.2 `api/failure_event_stream` を `api/supervision/escalation` に統合

#### 変更手順
```bash
# 1. failure_event_stream を escalation に統合
cat modules/actor-core/src/api/failure_event_stream.rs \
    >> modules/actor-core/src/api/supervision/escalation/failure_event_stream.rs

# 2. 古いファイル削除
git rm modules/actor-core/src/api/failure_event_stream.rs

# 3. tests も移動
git mv modules/actor-core/src/api/failure_event_stream/tests.rs \
       modules/actor-core/src/api/supervision/escalation/tests/failure_event_stream_tests.rs
```

#### `api.rs` 修正
```rust
// Before
pub mod failure_event_stream;

// After
// (削除 - supervision/escalation に統合)
```

#### `api/supervision/escalation.rs` 修正
```rust
// 追加
pub mod failure_event_stream;

pub use failure_event_stream::*;
```

### Phase 4 完了チェックリスト
- [ ] すべてのテストがパス
- [ ] `api/actor/failure` 削除完了
- [ ] `api/failure_event_stream` 削除完了
- [ ] import 文の修正完了
- [ ] git コミット: `git commit -m "refactor(actor-core): Phase 4 - supervision consolidation"`

## Phase 5: 最終調整(優先度: Low)

### 5.1 `api/actor/ask` を `api/ask` に昇格

#### 理由
Ask パターンは重要な機能であり、独立モジュールとして発見しやすくすべき

#### 変更手順
```bash
# 1. ask モジュールを api 直下に移動
git mv modules/actor-core/src/api/actor/ask \
       modules/actor-core/src/api/ask

# 2. api.rs 修正
# Before: pub mod actor; (内部に ask がある)
# After: pub mod ask; (独立)
```

#### `api.rs` 修正
```rust
// 追加
pub mod ask;
```

#### `api/actor.rs` 修正
```rust
// 削除
// pub mod ask;
```

#### `lib.rs` での re-export 維持
```rust
// Re-exports (変更なし - 既に api::actor::ask:: から公開されている)
pub use api::ask::{AskError, AskFuture, AskTimeoutFuture};
```

### 5.2 `api/actor_system/spawn` を `api/actor/spawn` に移動

#### 理由
アクター生成は actor モジュールの責務

#### 変更手順
```bash
git mv modules/actor-core/src/api/actor_system/spawn.rs \
       modules/actor-core/src/api/actor/spawn.rs
```

#### `api/actor_system.rs` 修正
```rust
// 削除
// pub mod spawn;
```

#### `api/actor.rs` 修正
```rust
// 追加
pub mod spawn;

pub use spawn::*;
```

### 5.3 `api/runtime` モジュール作成

#### 統合対象
- `api/actor_runtime/*` → `api/runtime/*`
- `api/mailbox/mailbox_runtime.rs` → `api/runtime/mailbox_runtime.rs`

#### 変更手順
```bash
# 1. runtime ディレクトリ作成
mkdir -p modules/actor-core/src/api/runtime

# 2. actor_runtime を runtime に移動
git mv modules/actor-core/src/api/actor_runtime/generic_actor_runtime.rs \
       modules/actor-core/src/api/runtime/actor_runtime.rs

# 3. mailbox_runtime を runtime に移動
git mv modules/actor-core/src/api/mailbox/mailbox_runtime.rs \
       modules/actor-core/src/api/runtime/mailbox_runtime.rs

# 4. runtime.rs 作成
cat > modules/actor-core/src/api/runtime/runtime.rs << 'EOF'
pub mod actor_runtime;
pub mod mailbox_runtime;

pub use actor_runtime::*;
pub use mailbox_runtime::*;
EOF

# 5. 古いディレクトリ削除
git rm -r modules/actor-core/src/api/actor_runtime/
```

#### `api.rs` 修正
```rust
// Before
pub mod actor_runtime;

// After
pub mod runtime;
```

### Phase 5 完了チェックリスト
- [ ] すべてのテストがパス
- [ ] `api/ask` が独立モジュールとして動作
- [ ] `api/runtime` が統一されている
- [ ] `api/actor/spawn` が正しく動作
- [ ] 最終的なファイル数確認: 137ファイル
- [ ] git コミット: `git commit -m "refactor(actor-core): Phase 5 - final adjustments"`

## 移行完了後の検証

### 完全性チェック
```bash
# 1. すべてのテスト実行
cargo test --workspace --all-features

# 2. すべてのサンプル実行
for example in examples/*.rs; do
    cargo run --example $(basename $example .rs)
done

# 3. ベンチマーク実行
cargo bench --workspace

# 4. ドキュメント生成
cargo doc --no-deps --workspace

# 5. CI スクリプト
./scripts/ci.sh all
```

### メトリクス確認
```bash
# ファイル数確認
find modules/actor-core/src -name "*.rs" | wc -l
# 期待値: 137ファイル

# モジュール数確認
find modules/actor-core/src -type f -name "*.rs" -path "*/mod.rs" | wc -l
find modules/actor-core/src -type f -name "*.rs" | \
  xargs grep -l "^pub mod " | wc -l

# コード行数確認
tokei modules/actor-core/src
```

### 後方互換性確認
```bash
# Deprecated 警告の確認
cargo build --workspace 2>&1 | grep "warning: use of deprecated"

# 外部依存パッケージのビルド確認
cargo build -p cellex-actor-std-rs
cargo build -p cellex-actor-embedded-rs
cargo build -p cellex-actor-remote-rs
```

## Deprecation スケジュール

### Version 2.0.0 (移行開始)
- すべての新しいパスを追加
- 古いパスに `#[deprecated]` 属性を付与
- Deprecation 警告を有効化

### Version 2.1.0 (3ヶ月後)
- Deprecation 警告のレベルを上げる
- ドキュメントに移行ガイドを明記
- サンプルコードをすべて新しいパスに更新

### Version 2.2.0 (6ヶ月後)
- Deprecated パスを削除予定として告知
- CI で deprecated パスの使用をエラーに変更

### Version 3.0.0 (12ヶ月後)
- Deprecated パスを完全削除
- Breaking change としてリリース

## トラブルシューティング

### import エラーが発生した場合
```bash
# すべての import 文を検索
grep -r "use crate::" modules/actor-core/src/ | grep -v "//" > imports.txt

# パターン置換スクリプト実行
./scripts/fix-imports.sh
```

### テストが失敗した場合
```bash
# 失敗したテストのみ実行
cargo test --workspace -- --nocapture <test_name>

# verbose モードで詳細確認
RUST_BACKTRACE=1 cargo test --workspace -- --nocapture
```

### Circular dependency エラー
```bash
# 循環依存の検出
cargo modules dependencies --lib | grep "circular"

# 依存グラフの可視化
cargo depgraph | dot -Tpng > depgraph.png
open depgraph.png
```

## まとめ

この移行ガイドに従うことで:
- ✅ 18ファイル(11.6%)削減
- ✅ モジュール構造の簡略化
- ✅ 後方互換性の維持
- ✅ 段階的な移行によるリスク最小化
- ✅ 12ヶ月の移行期間

各フェーズの完了チェックリストを確実に実施し、問題が発生した場合は前のフェーズに戻ることができます。

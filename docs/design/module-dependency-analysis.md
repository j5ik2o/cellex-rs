# モジュール依存関係分析

## 依存関係の原則

### レイヤー間依存ルール
```
api → internal → shared
  ↑       ↑         ↑
  |       |         |
外部 ← lib.rs (re-exports)
```

**許可される依存**:
- ✅ `api` → `internal` (公開APIが内部実装を使用)
- ✅ `internal` → `shared` (内部実装が共有ユーティリティを使用)
- ✅ `api` → `shared` (公開APIが共有型を使用)
- ❌ `internal` → `api` (循環依存の防止)
- ❌ `shared` → `api` or `internal` (共有層は最下層)

### モジュール内依存ルール
- 同一レイヤー内のモジュール間依存は最小化
- サブモジュールは親モジュールを通じて公開
- FQCN (Fully Qualified Class Name) imports を使用

## 現在の依存関係マップ

### API層の依存関係

#### `api/actor` の依存
```
api/actor/
├─→ internal/actor          (ActorCell, InternalProps)
├─→ internal/context        (ActorContext)
├─→ internal/scheduler      (ActorScheduler, SchedulerSpawnContext)
├─→ api/messaging           (DynMessage, MessageEnvelope)
├─→ api/mailbox             (Mailbox, MailboxSignal)
├─→ api/supervision         (Supervisor, FailureEvent)
├─→ api/extensions          (Extensions)
└─→ shared/failure_telemetry (FailureTelemetryShared)
```

**問題点**:
- `api/actor` が多くの内部実装に直接依存
- `actor_ref`, `behavior`, `context` 間に循環依存の可能性

#### `api/actor_system` の依存
```
api/actor_system/
├─→ internal/actor_system   (InternalActorSystem, InternalConfig)
├─→ internal/scheduler      (SchedulerBuilder)
├─→ internal/metrics        (MetricsSink)
├─→ api/actor               (ActorRef, Context, Props)
├─→ api/actor_runtime       (ActorRuntime)
├─→ api/extensions          (Extensions)
└─→ shared/map_system       (MapSystemShared)
```

**問題点**:
- `spawn` trait が `api/actor_system` にあるが、`api/actor` と強く結合

#### `api/messaging` の依存
```
api/messaging/
├─→ internal/message        (InternalMessageMetadata, MetadataTable)
├─→ api/extensions          (SerializerRegistry)
└─→ shared (なし)
```

**評価**: ✅ 依存関係が明確でクリーン

#### `api/mailbox` の依存
```
api/mailbox/
├─→ internal/mailbox        (PriorityMailboxBuilder, Spawner)
├─→ api/messaging           (DynMessage, MessageEnvelope)
├─→ api/actor_runtime       (MailboxRuntime)
└─→ cellex-utils-core-rs    (Queue, Signal)
```

**問題点**:
- `MailboxRuntime` が `api/actor_runtime` にあるが、論理的には `api/mailbox` の一部

#### `api/supervision` の依存
```
api/supervision/
├─→ internal/supervision    (EscalationSink implementations)
├─→ internal/guardian       (GuardianStrategy)
├─→ api/actor               (ActorRef, Context)
├─→ api/messaging           (MessageMetadata)
├─→ api/failure_event_stream (FailureEventStream)
└─→ shared/failure_telemetry (FailureTelemetryShared)
```

**問題点**:
- `api/failure_event_stream` が独立モジュールになっているが、論理的には `supervision` の一部
- `api/actor/failure` との責務重複

#### `api/extensions` の依存
```
api/extensions/
└─→ cellex-serialization-core-rs (Serializer trait)
```

**評価**: ✅ 外部クレートのみに依存、クリーン

### Internal層の依存関係

#### `internal/actor` の依存
```
internal/actor/
├─→ api/actor               (Props, Behavior, Context)
├─→ api/mailbox             (Mailbox)
└─→ internal/context        (ActorContext)
```

**問題点**:
- ⚠️ `internal` → `api` の依存(循環依存のリスク)

#### `internal/actor_system` の依存
```
internal/actor_system/
├─→ api/actor_system        (ActorSystemConfig)
├─→ api/actor               (RootContext)
├─→ internal/scheduler      (SchedulerBuilder)
└─→ internal/metrics        (MetricsSink)
```

**問題点**:
- ⚠️ `internal` → `api` の依存(循環依存のリスク)

#### `internal/context` の依存
```
internal/context/
├─→ api/actor               (Context, ActorRef, Behavior)
├─→ api/messaging           (DynMessage)
├─→ api/supervision         (Supervisor)
└─→ internal/scheduler      (SchedulerSpawnContext)
```

**問題点**:
- ⚠️ `internal` → `api` の依存(循環依存のリスク)

#### `internal/scheduler` の依存
```
internal/scheduler/
├─→ api/actor               (Props, Context)
├─→ api/mailbox             (Mailbox, MailboxSignal)
├─→ api/extensions          (Extensions)
├─→ internal/guardian       (GuardianStrategy)
└─→ shared/receive_timeout  (ReceiveTimeoutDriver)
```

**問題点**:
- ⚠️ `internal` → `api` の依存(循環依存のリスク)
- `ready_queue_scheduler/*` 内部でのサブモジュール間依存が複雑

#### `internal/mailbox` の依存
```
internal/mailbox/
├─→ api/mailbox             (Mailbox, MailboxOptions, MailboxRuntime)
└─→ cellex-utils-core-rs    (Queue)
```

**問題点**:
- ⚠️ `internal` → `api` の依存

#### `internal/message` の依存
```
internal/message/
├─→ api/messaging           (MessageMetadata, DynMessage)
└─→ shared (なし)
```

**評価**: ✅ 依存関係がシンプル(api へは許容範囲)

#### `internal/metrics` の依存
```
internal/metrics/
└─→ (外部依存のみ)
```

**評価**: ✅ 完全に独立、優れた設計

#### `internal/guardian` の依存
```
internal/guardian/
├─→ api/actor               (ActorRef, Context)
├─→ api/supervision         (Supervisor, FailureInfo)
└─→ internal/scheduler      (SchedulerSpawnContext)
```

**評価**: ✅ 許容範囲の依存

#### `internal/supervision` の依存
```
internal/supervision/
├─→ api/supervision         (FailureEvent, EscalationSink)
├─→ internal/guardian       (GuardianStrategy)
└─→ shared/failure_telemetry (FailureTelemetryShared)
```

**評価**: ✅ 許容範囲の依存

### Shared層の依存関係

#### `shared/failure_telemetry` の依存
```
shared/failure_telemetry/
└─→ api/supervision/telemetry (FailureTelemetry trait)
```

**問題点**:
- ⚠️ `shared` → `api` の依存(設計原則違反)
- 本来 `shared` は最下層で他に依存すべきでない

#### `shared/receive_timeout` の依存
```
shared/receive_timeout/
└─→ (外部依存のみ)
```

**評価**: ✅ 完全に独立

#### `shared/map_system` の依存
```
shared/map_system/
└─→ api/mailbox/messages    (SystemMessage)
```

**問題点**:
- ⚠️ `shared` → `api` の依存(設計原則違反)

## 循環依存の検出

### 検出された循環依存

#### 1. `api/actor` ⇄ `internal/actor`
```
api/actor/props.rs
  → internal/actor/internal_props.rs
    → api/actor/behavior.rs
      → api/actor/context.rs (循環)
```

**影響**: 中程度
**解決策**: `InternalProps` を完全に `internal` に隠蔽し、`api/actor/props` のみを公開

#### 2. `api/actor` ⇄ `internal/context`
```
api/actor/context.rs
  → internal/context/actor_context.rs
    → api/actor/behavior.rs (循環)
```

**影響**: 高
**解決策**: `Context` trait を `api` に、`ActorContext` 実装を `internal` に完全分離

#### 3. `api/supervision` ⇄ `shared/failure_telemetry`
```
api/supervision/telemetry.rs
  → shared/failure_telemetry/failure_telemetry_shared.rs
    → api/supervision/telemetry.rs (循環)
```

**影響**: 中程度
**解決策**: `shared/failure_telemetry` を `internal/supervision/telemetry` に移動

### 検出されていないが潜在的な循環依存

#### 4. `api/mailbox` ⇄ `api/actor_runtime`
```
api/mailbox/mailbox_runtime.rs
  → api/actor_runtime/generic_actor_runtime.rs
    → api/mailbox/* (間接的循環)
```

**影響**: 低
**解決策**: `MailboxRuntime` を `api/runtime` に統合

## 依存関係の問題まとめ

### Critical (即座に対処が必要)
1. **`shared` → `api` の依存** (2箇所)
   - `shared/failure_telemetry` → `api/supervision`
   - `shared/map_system` → `api/mailbox`
   - **解決**: `shared` から `internal` に移動

2. **`api/actor` ⇄ `internal/context` の循環依存**
   - **解決**: Context trait を完全に `api` に、実装を `internal` に分離

### High (優先的に対処)
3. **`internal` → `api` の広範な依存**
   - 多くの `internal` モジュールが `api` に依存
   - **解決**: trait-based 抽象化を導入し、依存方向を逆転

4. **`api/actor/failure` と `api/supervision/failure` の重複**
   - **解決**: `api/supervision/failure` に統合

### Medium (計画的に対処)
5. **Scheduler の複雑な依存関係**
   - `internal/scheduler` の14サブモジュール間の依存が複雑
   - **解決**: サブモジュールを統合し、依存を簡略化

6. **Messaging/Mailbox の責務分散**
   - `api/messaging`, `api/mailbox/messages`, `internal/message` の役割が不明確
   - **解決**: 責務を明確化し、統合

## 理想的な依存関係フロー

### レイヤー分離の徹底
```rust
// ✅ Good: api → internal
// api/actor/props.rs
use crate::internal::actor::InternalProps;

// ❌ Bad: internal → api
// internal/actor/actor_cell.rs
use crate::api::actor::Props;  // 循環依存のリスク
```

### Trait-based 抽象化
```rust
// ✅ Good: internal が api の trait を実装
// api/actor/context.rs
pub trait Context { ... }

// internal/context/actor_context.rs
use crate::api::actor::Context;
impl Context for ActorContext { ... }
```

### Shared層の完全独立
```rust
// ✅ Good: shared は外部クレートのみに依存
// shared/receive_timeout/driver.rs
use cellex_utils_core_rs::...;

// ❌ Bad: shared が api に依存
// shared/failure_telemetry/telemetry.rs
use crate::api::supervision::telemetry::FailureTelemetry;
```

## 依存関係改善のロードマップ

### Phase 1: Critical 問題の解決
1. `shared/failure_telemetry` を `internal/supervision/telemetry` に移動
2. `shared/map_system` を `internal/message` に移動
3. `api/actor` ⇄ `internal/context` の循環を解消

### Phase 2: High 問題の解決
4. `internal` → `api` 依存を trait-based に変更
5. `api/actor/failure` を `api/supervision/failure` に統合

### Phase 3: Medium 問題の解決
6. Scheduler のサブモジュールを統合
7. Messaging/Mailbox の責務を再編成

### Phase 4: 最適化
8. モジュール間の不要な依存を削除
9. 依存関係グラフの可視化
10. CI で循環依存を検出する仕組みを導入

## 依存関係の可視化

### ツール推奨
```bash
# cargo-modules で依存関係を可視化
cargo install cargo-modules
cargo modules generate graph --lib | dot -Tpng > modules.png

# cargo-depgraph で依存グラフを生成
cargo install cargo-depgraph
cargo depgraph | dot -Tpng > depgraph.png
```

### CI Integration
```yaml
# .github/workflows/check-dependencies.yml
name: Check Module Dependencies
on: [push, pull_request]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Check for circular dependencies
        run: cargo modules dependencies --lib | grep "circular" && exit 1 || exit 0
```

## まとめ

現在のモジュール構成には以下の依存関係問題があります:

1. **2箇所の `shared` → `api` 依存** (設計原則違反)
2. **3箇所の循環依存** (保守性低下)
3. **広範な `internal` → `api` 依存** (レイヤー分離不足)
4. **責務の重複** (failure, messaging 関連)

これらを段階的に解決することで、より保守しやすく拡張性の高いアーキテクチャを実現できます。

# Actor Scheduler Refactoring - 実装計画

**作成日**: 2025-10-22
**ベースドキュメント**: `docs/design/actor_scheduler_refactor.md`

## 1. 現状分析

### 1.1 Phase 1 完了状況 ✅

**実装済みコンポーネント**:
- `ReadyQueueCoordinator` トレイト（V1: `&mut self`メソッド）
- `ReadyQueueCoordinatorV2` トレイト（V2: `&self`メソッド、interior mutability）
- `DefaultReadyQueueCoordinator` / `DefaultReadyQueueCoordinatorV2`
- `LockFreeCoordinator` / `LockFreeCoordinatorV2`（2.2x性能改善達成）
- `AdaptiveCoordinator`
- 関連型定義: `InvokeResult`, `SuspendReason`, `ResumeCondition`, `MailboxIndex`, `ActorState`

**Phase 1 目標達成**:
- ✅ ReadyQueueCoordinatorの抽出と新実装
- ✅ lock-free実装による性能改善（2.2x）
- ✅ テストとベンチマーク整備

**ギャップ**:
- デザインドキュメントでは`SmallVec<[MailboxIndex; 64]>`を提案
- 現実装では`Vec<MailboxIndex>`を使用
- **影響**: 軽微（最適化として後で対応可能）

### 1.2 Phase 2A 未実装状況 ❌

**デザイン提案**:
```rust
pub trait RuntimeHandle {
    fn spawn(&self, task: impl Future<Output = ()>);
    fn invoke(&self, idx: MailboxIndex) -> impl Future<Output = InvokeResult>;
    fn wait_with(&self, f: impl FnOnce(&mut Context) -> Poll<()>) -> impl Future<Output = ()>;
}

pub struct WorkerExecutor<R: ReadyQueueCoordinator, H: RuntimeHandle> {
    coordinator: ArcShared<R>,
    runtime: H,
    worker_batch: usize,
}
```

**現状**:
- `drive_ready_queue_worker()` 関数（関数ベース）
- `ReadyQueueWorker` トレイト
- `ReadyQueueWorkerImpl` 構造体

**ギャップ**:
- `RuntimeHandle` トレイトがない
- `WorkerExecutor` 構造体として構造化されていない
- Tokio/Embassy/テストランタイムの抽象化がない

### 1.3 Phase 2B 未実装状況 ❌

**デザイン提案**:
```rust
pub trait MessageInvoker: Send {
    fn invoke_batch(&mut self, max_messages: usize) -> InvokeResult;
    fn actor_state(&self) -> ActorState;
}
```

**現状**:
- `ReadyQueueContext::process_ready_once()` が責務を持つ
- `ReadyQueueSchedulerCore::process_actor_pending()` が実際の処理

**ギャップ**:
- `MessageInvoker` トレイトがない
- ActorCellに処理が埋め込まれている
- Suspend/Resume、middleware、backpressureが抽象化されていない

### 1.4 Phase 3 未実装状況 ❌

**デザイン提案**:
```rust
pub trait MailboxRegistry: Send + Sync {
    fn register_mailbox(&mut self, cell: Arc<ActorCell>, mailbox: Arc<QueueMailbox>) -> MailboxIndex;
    fn get_mailbox(&self, idx: MailboxIndex) -> Option<Arc<QueueMailbox>>;
    fn get_actor_cell(&self, idx: MailboxIndex) -> Option<Arc<ActorCell>>;
    fn unregister_mailbox(&mut self, idx: MailboxIndex) -> bool;
}
```

**現状**:
- MailboxRegistryは存在しない
- `ReadyQueueSchedulerCore` が一部の責務を持つ

**ギャップ**:
- MailboxRegistry トレイトがない
- Observability Hubがない
- ライフサイクル管理が分散している

## 2. 実装ロードマップ

### 2.1 Phase 2A: WorkerExecutor の導入

**目標**: ランタイム依存のタスク生成とワーカー駆動を抽象化

**実装ステップ**:

#### Step 2A-1: RuntimeHandle トレイトの定義
**ファイル**: `modules/actor-core/src/api/actor_scheduler/runtime_handle.rs`

```rust
use core::future::Future;
use core::task::{Context, Poll};
use cellex_utils_core_rs::sync::SendBound;

/// Runtime-specific task spawning and execution abstraction
pub trait RuntimeHandle: Clone + SendBound + 'static {
    /// Spawn a new async task on the runtime
    fn spawn<F>(&self, task: F)
    where
        F: Future<Output = ()> + SendBound + 'static;

    /// Wait for a condition using poll-based future
    fn wait_with<F>(&self, f: F) -> impl Future<Output = ()>
    where
        F: FnOnce(&mut Context<'_>) -> Poll<()>;
}
```

**実装ターゲット**:
- `TokioRuntimeHandle` (actor-std)
- `EmbassyRuntimeHandle` (actor-embedded)
- `TestRuntimeHandle` (actor-core, test feature)

#### Step 2A-2: WorkerExecutor 構造体の実装
**ファイル**: `modules/actor-core/src/api/actor_scheduler/worker_executor.rs`

```rust
use alloc::vec::Vec;
use cellex_utils_core_rs::sync::ArcShared;

use super::{ReadyQueueCoordinatorV2, RuntimeHandle, MailboxIndex, InvokeResult};

pub struct WorkerExecutor<C, H>
where
    C: ReadyQueueCoordinatorV2 + 'static,
    H: RuntimeHandle,
{
    coordinator: ArcShared<C>,
    runtime: H,
    worker_batch: usize,
}

impl<C, H> WorkerExecutor<C, H>
where
    C: ReadyQueueCoordinatorV2 + 'static,
    H: RuntimeHandle,
{
    pub fn new(coordinator: ArcShared<C>, runtime: H, worker_batch: usize) -> Self {
        Self {
            coordinator,
            runtime,
            worker_batch,
        }
    }

    pub fn start_workers(&self, num_workers: usize) {
        for worker_id in 0..num_workers {
            let coordinator = self.coordinator.clone();
            let runtime = self.runtime.clone();
            let max_batch = self.worker_batch;

            self.runtime.spawn(async move {
                Self::worker_loop(worker_id, coordinator, runtime, max_batch).await;
            });
        }
    }

    async fn worker_loop(
        worker_id: usize,
        coordinator: ArcShared<C>,
        runtime: H,
        max_batch: usize,
    ) {
        let mut buffer = Vec::with_capacity(max_batch);

        loop {
            // Wait for signal
            runtime.wait_with(|cx| coordinator.poll_wait_signal(cx)).await;

            // Drain ready mailboxes
            buffer.clear();
            coordinator.drain_ready_cycle(max_batch, &mut buffer);

            // Process each mailbox
            for idx in buffer.iter().copied() {
                // TODO: invoke MessageInvoker here (Phase 2B)
                // let result = invoke_mailbox(idx).await;
                // coordinator.handle_invoke_result(idx, result);
            }
        }
    }
}
```

#### Step 2A-3: 既存コードとの統合
**変更ファイル**:
- `modules/actor-core/src/api/actor_scheduler/ready_queue_scheduler/base.rs`
  - `WorkerExecutor` を使用するように変更
- `modules/actor-core/src/api/actor_system/base.rs`
  - `RuntimeHandle` を提供するように変更

**feature flag**: `new-scheduler` で新実装を有効化

**テスト**:
- `WorkerExecutor` の単体テスト
- Tokio/テストランタイムでの統合テスト（15ケース）
- レイテンシ劣化 Phase 1比で +3%以内を確認

### 2.2 Phase 2B: MessageInvoker の導入

**目標**: ActorCellからメッセージ実行ループを分離

**実装ステップ**:

#### Step 2B-1: MessageInvoker トレイトの定義
**ファイル**: `modules/actor-core/src/api/actor_scheduler/message_invoker.rs`

```rust
use super::{InvokeResult, ActorState};

/// Abstracts message execution loop with suspend/resume support
pub trait MessageInvoker: SendBound {
    /// Process up to max_messages from the mailbox
    fn invoke_batch(&mut self, max_messages: usize) -> InvokeResult;

    /// Get current actor state
    fn actor_state(&self) -> ActorState;
}
```

#### Step 2B-2: ActorCellInvoker 実装
**ファイル**: `modules/actor-core/src/internal/actor/actor_cell_invoker.rs`

```rust
pub struct ActorCellInvoker<MF, Strat>
where
    MF: MailboxFactory + Clone + 'static,
    Strat: GuardianStrategy<MF>,
{
    cell: /* ActorCell reference */,
    throughput: usize,
}

impl<MF, Strat> MessageInvoker for ActorCellInvoker<MF, Strat>
where
    MF: MailboxFactory + Clone + 'static,
    Strat: GuardianStrategy<MF>,
{
    fn invoke_batch(&mut self, max_messages: usize) -> InvokeResult {
        // Check suspension state
        // Call middleware before_invoke
        // Process messages
        // Call middleware after_invoke
        // Return appropriate InvokeResult
    }

    fn actor_state(&self) -> ActorState {
        // Return current actor state
    }
}
```

#### Step 2B-3: WorkerExecutor と MessageInvoker の統合
**変更ファイル**: `modules/actor-core/src/api/actor_scheduler/worker_executor.rs`

```rust
async fn worker_loop(
    worker_id: usize,
    coordinator: ArcShared<C>,
    runtime: H,
    max_batch: usize,
    invoker_factory: /* InvokerFactory */,
) {
    let mut buffer = Vec::with_capacity(max_batch);

    loop {
        runtime.wait_with(|cx| coordinator.poll_wait_signal(cx)).await;

        buffer.clear();
        coordinator.drain_ready_cycle(max_batch, &mut buffer);

        for idx in buffer.iter().copied() {
            let mut invoker = invoker_factory.create(idx);
            let throughput = coordinator.throughput_hint();
            let result = invoker.invoke_batch(throughput);
            coordinator.handle_invoke_result(idx, result);
        }
    }
}
```

**テスト**:
- Suspend/Resume 統合テスト
- Middleware 連携テスト（7ケース）
- Guardian連携テスト（5ケース）
- Backpressureテスト（5ケース）
- 合計25ケース以上

### 2.3 Phase 3: MailboxRegistry と Observability Hub

**目標**: Mailboxライフサイクル管理とメトリクス統一

**実装ステップ**:

#### Step 3-1: MailboxRegistry トレイトの定義
**ファイル**: `modules/actor-core/src/api/actor_scheduler/mailbox_registry.rs`

```rust
pub trait MailboxRegistry: SendBound + SharedBound {
    type Cell;
    type Mailbox;

    fn register_mailbox(
        &mut self,
        cell: ArcShared<Self::Cell>,
        mailbox: ArcShared<Self::Mailbox>,
    ) -> MailboxIndex;

    fn get_mailbox(&self, idx: MailboxIndex) -> Option<ArcShared<Self::Mailbox>>;
    fn get_actor_cell(&self, idx: MailboxIndex) -> Option<ArcShared<Self::Cell>>;
    fn unregister_mailbox(&mut self, idx: MailboxIndex) -> bool;
}
```

#### Step 3-2: DefaultMailboxRegistry 実装
**ファイル**: `modules/actor-core/src/api/actor_scheduler/default_mailbox_registry.rs`

```rust
pub struct DefaultMailboxRegistry<MF, Strat>
where
    MF: MailboxFactory + Clone + 'static,
    Strat: GuardianStrategy<MF>,
{
    cells: Vec<Option<ArcShared<ActorCell<MF, Strat>>>>,
    mailboxes: Vec<Option<ArcShared</* Mailbox */>>>,
    free_indices: Vec<MailboxIndex>,
}
```

#### Step 3-3: Observability Hub 実装
**ファイル**: `modules/actor-core/src/api/actor_scheduler/observability_hub.rs`

```rust
pub struct ObservabilityHub {
    metrics_sink: Option<MetricsSinkShared>,
    // Lock-free metrics collection
}

impl ObservabilityHub {
    pub fn on_enqueue(&self, idx: MailboxIndex, priority: i8) { /* ... */ }
    pub fn on_dequeue(&self, idx: MailboxIndex) { /* ... */ }
    pub fn on_message_processed(&self, idx: MailboxIndex, duration: Duration) { /* ... */ }
}
```

**テスト**:
- MailboxRegistry統合テスト（10ケース）
- no_stdターゲット検証（thumbv6m-none-eabi, thumbv8m.main-none-eabi）
- QEMU + Embassy executor軽量テスト（3アクター × 100メッセージ）
- メトリクス送出がlock-freeであることを確認

## 3. 実装順序と依存関係

```
Phase 1 (完了)
    ↓
Phase 2A: RuntimeHandle + WorkerExecutor
    ↓
Phase 2B: MessageInvoker + ActorCellInvoker
    ↓ (2Aと2Bを統合)
Phase 3: MailboxRegistry + Observability Hub
    ↓
Phase 4: 統合・最適化・ドキュメント化
```

## 4. Feature Flag 戦略

**feature flags**:
- `new-scheduler`: 新実装全体を有効化
- `scheduler-phase-2a`: WorkerExecutor実装を有効化
- `scheduler-phase-2b`: MessageInvoker実装を有効化
- `scheduler-phase-3`: MailboxRegistry + Observability Hub実装を有効化

**デフォルト切り替え**: Phase 4完了後、1週間のステージング観測を経てデフォルト化

**ロールバック**: 各フェーズでfeature flagを無効化して旧実装に戻せる

## 5. パフォーマンス目標

**ベースライン**: Phase 0（現行ReadyQueueScheduler実装）

**許容値**:
- Phase 2A: レイテンシ +3%以内、スループット 95%以上維持
- Phase 2B: レイテンシ Phase 1比で追加 +7%以内（Phase 0比で合計 +10%以内）
- Phase 3: レイテンシ Phase 0比で +5%以内へ回復、スループット 95%回復

**計測方法**:
- `cargo bench --bench mailbox_throughput`
- `cargo bench --bench scheduler_latency`
- 3回測定して中央値を採用
- `scripts/compare_benchmarks.py` で自動比較

## 6. テスト戦略

**Phase 2A テスト**:
- RuntimeHandle実装テスト（Tokio/Test）
- WorkerExecutor単体テスト
- 統合テスト 15ケース
- ベンチマーク比較

**Phase 2B テスト**:
- MessageInvoker実装テスト
- Suspend/Resume統合テスト
- Middleware連携テスト（7ケース）
- Guardian連携テスト（5ケース）
- Backpressureテスト（5ケース）
- 合計25ケース以上

**Phase 3 テスト**:
- MailboxRegistry統合テスト（10ケース）
- Observability Hub統合テスト
- no_std検証（cargo check）
- QEMU軽量テスト

## 7. マイルストーン

| マイルストーン | 完了条件 | 期待日数 |
|--------------|---------|---------|
| Phase 2A-1: RuntimeHandle実装 | トレイト定義、Tokio/Test実装完了 | 2日 |
| Phase 2A-2: WorkerExecutor実装 | 構造体実装、テスト15ケース完了 | 3日 |
| Phase 2A-3: 統合とベンチマーク | レイテンシ目標達成、CI通過 | 2日 |
| Phase 2B-1: MessageInvoker定義 | トレイト定義完了 | 1日 |
| Phase 2B-2: ActorCellInvoker実装 | 実装とテスト25ケース完了 | 4日 |
| Phase 2B-3: WorkerExecutor統合 | 統合完了、ベンチマーク達成 | 2日 |
| Phase 3-1: MailboxRegistry | トレイト定義と実装完了 | 3日 |
| Phase 3-2: Observability Hub | 実装とテスト10ケース完了 | 2日 |
| Phase 3-3: no_std検証 | cargo check通過、QEMU テスト | 2日 |
| Phase 4: 統合・ドキュメント化 | 移行ガイド完成、旧実装削除 | 3日 |

**合計予想**: 約24日（約1ヶ月）

## 8. リスクと対策

| リスク | 影響 | 対策 |
|--------|------|------|
| パフォーマンス劣化 | 高 | 各Phaseでベンチマーク検証、feature flagでロールバック |
| Suspend/Resume設計複雑化 | 中 | Phase 2Bで集中対応、ADRで設計確定 |
| no_std対応漏れ | 中 | Phase 3でQEMUテスト、早期検証 |
| Guardianとの連携不具合 | 中 | Phase 2Bで回帰テスト強化 |

## 9. 次のアクション

1. ✅ 現状分析完了
2. ✅ 実装計画文書化完了
3. ⏭️ **Phase 2A-1**: RuntimeHandle トレイト定義の実装を開始

**開始ファイル**: `modules/actor-core/src/api/actor_scheduler/runtime_handle.rs`

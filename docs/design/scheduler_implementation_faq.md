# ActorScheduler リファクタリング実装 FAQ

## 概要

このドキュメントは、ActorScheduler リファクタリングにおいてよくある質問とその回答をまとめたものです。実装者が設計を正しく理解し、一貫性のあるコードを書くための参考資料として使用してください。

**最終更新**: 2025-10-22
**対象フェーズ**: Phase 0-4
**関連ドキュメント**: `actor_scheduler_refactor.md`, ADR-001, ADR-002

---

## カテゴリ

- **A1**: アーキテクチャ基礎 - 責務分担、コンポーネント関係
- **A2**: アーキテクチャ詳細 - 並行制御、ランタイム抽象、no_std対応
- **B1**: 実装 - トレイト実装、テスト戦略、コーディング規約
- **D1**: 開発/デバッグ - パフォーマンス測定、ロールバック、トラブルシュート

---

## A1: アーキテクチャ基礎

### Q1-1. ReadyQueueCoordinator、WorkerExecutor、MessageInvokerの責務分担は？

**A1-1.**

各コンポーネントは単一責任原則に従って責務を明確に分離しています：

| コンポーネント | 責務 | 所有データ |
|----------------|------|------------|
| **ReadyQueueCoordinator** | Ready queueの登録/除外/排出を管理 | `QueueState`（VecDeque + HashSet）、シグナルチャネル |
| **WorkerExecutor** | ワーカタスクのライフサイクル管理と駆動 | Coordinatorへの参照、Runtime固有のタスクハンドル |
| **MessageInvoker** | メッセージ実行ロジックとミドルウェアチェイン | Mailbox/ActorCellへの参照、ミドルウェアスタック |

**シーケンス**:
```
WorkerExecutor::run()
  → Coordinator::poll_wait_signal() (シグナル待機)
  → Coordinator::drain_ready_cycle() (インデックス取得)
  → MessageInvoker::invoke_batch() (メッセージ処理)
  → Coordinator::handle_invoke_result() (結果に応じて再登録/除外)
```

**参照**: `actor_scheduler_refactor.md` Section 4.1-4.3, `scheduler_message_flow.puml`

---

### Q1-2. `MailboxIndex` から実際の Mailbox をどう取得する？

**A1-2.**

`MailboxRegistry` が中央管理者として機能します：

```rust
// 登録時
let idx = mailbox_registry.register_mailbox(mailbox, actor_cell);
// idx = MailboxIndex { slot: 42, generation: 1 }

// 取得時
let mailbox = mailbox_registry.get_mailbox(idx)?;
let actor_cell = mailbox_registry.get_actor_cell(idx)?;
```

**キャッシュ戦略**（Phase 2B以降）:
```rust
impl MessageInvoker {
  fn new(idx: MailboxIndex, registry: Arc<MailboxRegistry>) -> Self {
    // 初期化時にキャッシュ
    let mailbox = registry.get_mailbox(idx).expect("valid index");
    let actor_cell = registry.get_actor_cell(idx).expect("valid index");

    Self {
      idx,
      mailbox,      // Arc<QueueMailbox> をキャッシュ
      actor_cell,   // Arc<ActorCell> をキャッシュ
      // ...
    }
  }

  fn invoke_batch(&mut self, throughput_hint: usize) -> InvokeResult {
    // ホットパスではキャッシュを使用（ロックなし）
    self.mailbox.dequeue_batch(throughput_hint)
    // ...
  }
}
```

**世代管理**（Phase 1）:
- `MailboxIndex { slot: 42, generation: 1 }` の `generation` により、再利用時のuse-after-freeを防止
- `get_mailbox(idx)` は世代不一致の場合 `None` を返す

**参照**: `actor_scheduler_refactor.md` Section 4.8, `mailbox_registry_generational.md` (Phase 1)

---

### Q1-3. Suspend/Resume の責務はどのコンポーネントが担う？

**A1-3.**

責務は以下のように分担されます（ADR-002で決定）：

| フェーズ | MessageInvoker | ReadyQueueCoordinator | ActorCell | MailboxRegistry |
|---------|---------------|----------------------|-----------|----------------|
| **Suspend判定** | `actor_state()`を評価 | - | 自身の状態を`Suspended`に設定 | - |
| **Suspend通知** | `InvokeResult::Suspended`を返却 | `unregister(idx)`を実行 | - | - |
| **Resume契機** | - | - | タイマー/シグナル/容量監視 | - |
| **Resume通知** | - | `register_ready(idx)`で再登録 | - | `notify_resume(idx)`を仲介 |

**Suspendフロー**:
```
MessageInvoker
  ↓ check actor_state() → Suspended
  ↓ return InvokeResult::Suspended { reason, resume_on }
ReadyQueueCoordinator
  ↓ handle_invoke_result(idx, Suspended)
  ↓ unregister(idx)
```

**Resumeフロー**:
```
ActorCell (Resume契機を検出)
  ↓ set_state(Running)
  ↓ registry.notify_resume(idx)
MailboxRegistry
  ↓ coordinator.register_ready(idx)
ReadyQueueCoordinator
  ↓ register_ready(idx) (ready queueに再登録)
```

**参照**: `docs/adr/2025-10-22-phase0-suspend-resume-responsibility.md`, `scheduler_sequences.puml` (Backpressure flow)

---

### Q1-4. ReadyQueueCoordinator と WorkerExecutor はどちらがメインループを持つ？

**A1-4.**

**WorkerExecutor がメインループを所有します**：

```rust
impl WorkerExecutor {
  pub async fn run(&self) {
    loop {
      // 1. シグナル待機
      self.coordinator.poll_wait_signal(&mut cx).await;

      // 2. Ready queueから取り出し
      let mut batch = Vec::new();
      self.coordinator.drain_ready_cycle(self.max_batch, &mut batch);

      // 3. 各インデックスに対してInvokerを実行
      for idx in batch {
        let invoker = self.create_invoker(idx);
        let result = invoker.invoke_batch(self.throughput_hint);

        // 4. 結果をCoordinatorに通知
        self.coordinator.handle_invoke_result(idx, result);
      }
    }
  }
}
```

**Coordinatorの役割**:
- `QueueState`（VecDeque + HashSet）への排他アクセスを提供
- `poll_wait_signal` / `drain_ready_cycle` / `handle_invoke_result` などのメソッドを通じて状態を操作
- **メソッド単位でロックを取得し、呼び出し後は即座に解放**

**参照**: `actor_scheduler_refactor.md` Section 4.7

---

## A2: アーキテクチャ詳細

### Q2-1. 並行アクセスの排他制御はどこで行う？

**A2-1.**

排他制御は各コンポーネントで以下のように実装されます：

| コンポーネント | 排他制御方式 | 保護対象 |
|----------------|-------------|---------|
| **ReadyQueueCoordinator** | `Arc<Mutex<QueueState>>` (Phase 0-1)<br>`DashSet` (Phase 2以降) | ready queue（VecDeque + HashSet） |
| **QueueMailbox** | `RwLock<Queues>` | system_queue + user_queue |
| **MailboxRegistry** | `DashMap<MailboxIndex, Entry>` | mailbox/actor_cell マッピング |
| **MessageInvoker** | なし（ローカルキャッシュのみ） | - |

**クリティカルセクション最小化戦略**:

```rust
impl ReadyQueueCoordinator for DefaultReadyQueueCoordinator {
  fn register_ready(&mut self, idx: MailboxIndex) {
    let mut state = self.state.lock().unwrap();
    // ロック保持中の処理を最小限に
    if state.queued.insert(idx) {
      state.queue.push_back(idx);
      state.signal_pending = true;
    }
    // スコープ終了でロック自動解放
  }
}
```

**Phase 2以降の最適化**:
- `DashSet<MailboxIndex>` によるlock-free重複検出
- `MPSC`チャネルによるシグナル通知（ロックフリー）
- per-workerローカルキューの検討（Phase 3）

**参照**: `actor_scheduler_refactor.md` Section 4.7, 5.2

---

### Q2-2. `cfg(feature = "new-scheduler")` の使い分けは？

**A2-2.**

Feature flag は段階的ロールアウトのために使用されます：

**Phase 0-3**:
```toml
# Cargo.toml
[features]
new-scheduler = ["std"]
```

```rust
// モジュール定義
#[cfg(feature = "new-scheduler")]
pub mod ready_queue_coordinator;

// 実装切り替え
#[cfg(feature = "new-scheduler")]
use crate::actor_scheduler::ready_queue_coordinator::DefaultReadyQueueCoordinator;
#[cfg(not(feature = "new-scheduler"))]
use crate::actor_scheduler::ready_queue_scheduler::ReadyQueueScheduler;
```

**Phase 4**:
- デフォルトで `new-scheduler` を有効化
- 1週間のステージング観測
- 旧実装コードを削除（feature flagも削除）

**テスト戦略**:
```bash
# 旧実装テスト
cargo test --workspace

# 新実装テスト
cargo test --workspace --features new-scheduler

# ベンチマーク比較
cargo bench --bench mailbox_throughput
cargo bench --bench mailbox_throughput --features new-scheduler
```

**参照**: `actor_scheduler_refactor.md` Section 3.1, 5.3

---

### Q2-3. no_std 環境ではどう対応する？

**A2-3.**

`alloc` ベースで実装し、`std` 依存機能は `Shared` 抽象を経由させます：

```rust
// 抽象定義
#[cfg(feature = "std")]
pub type Shared<T> = Arc<T>;

#[cfg(not(feature = "std"))]
pub type Shared<T> = alloc::sync::Arc<T>; // allocのArcを使用

// 使用例
pub struct DefaultReadyQueueCoordinator {
  state: Shared<Mutex<QueueState>>,  // Arc<Mutex<_>> or alloc::sync::Arc<Mutex<_>>
  throughput: usize,
}
```

**no_std 対応チェックリスト（Phase 3）**:

1. **ターゲット**: `thumbv6m-none-eabi`, `thumbv8m.main-none-eabi`
2. **ビルド確認**: `cargo check --target thumbv6m-none-eabi --no-default-features --features alloc`
3. **ランタイム**: Embassy executor での軽量統合テスト（3アクター × 100メッセージ）
4. **制約**:
   - `std::thread` → Embassy tasks
   - `Instant::now()` → ランタイム固有のタイマー抽象
   - `tokio::sync::*` → Embassy同期プリミティブ

**参照**: `actor_scheduler_refactor.md` Section 5.4

---

## B1: 実装

### Q3-1. ReadyQueueCoordinator トレイトを実装する手順は？

**B3-1.**

以下の手順で実装します：

**Step 1: トレイト実装の骨格を作成**
```rust
pub struct MyCoordinator {
  state: Arc<Mutex<QueueState>>,
  throughput: usize,
}

impl ReadyQueueCoordinator for MyCoordinator {
  fn register_ready(&mut self, idx: MailboxIndex) {
    // TODO: 実装
  }

  fn unregister(&mut self, idx: MailboxIndex) {
    // TODO: 実装
  }

  fn drain_ready_cycle(&mut self, max_batch: usize, out: &mut Vec<MailboxIndex>) {
    // TODO: 実装
  }

  fn poll_wait_signal(&mut self, cx: &mut Context<'_>) -> Poll<()> {
    // TODO: 実装
  }

  fn handle_invoke_result(&mut self, idx: MailboxIndex, result: InvokeResult) {
    // TODO: 実装
  }

  fn throughput_hint(&self) -> usize {
    self.throughput
  }
}
```

**Step 2: 単体テストを先に書く（TDD）**
```rust
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_register_ready_basic() {
    let mut coordinator = MyCoordinator::new(32);
    let idx = MailboxIndex::new(1, 0);

    coordinator.register_ready(idx);

    let mut out = Vec::new();
    coordinator.drain_ready_cycle(10, &mut out);

    assert_eq!(out.len(), 1);
    assert_eq!(out[0], idx);
  }

  #[test]
  fn test_duplicate_prevention() {
    // 重複登録を防止することを確認
  }

  #[test]
  fn test_handle_invoke_result_suspended() {
    // InvokeResult::Suspended で unregister されることを確認
  }

  // ... 全13テストケース
}
```

**Step 3: 実装（DefaultReadyQueueCoordinatorを参考に）**
```rust
fn register_ready(&mut self, idx: MailboxIndex) {
  let mut state = self.state.lock().unwrap();
  // HashSetで重複チェック
  if state.queued.insert(idx) {
    state.queue.push_back(idx);
    state.signal_pending = true;
  }
}
```

**Step 4: テストを実行**
```bash
cargo test -p cellex-actor-core-rs ready_queue_coordinator
```

**必須テストケース（Phase 1完了基準）**:
- 正常系: 8ケース（基本登録、排出、再登録、throughput_hint等）
- 異常系: 7ケース（重複、空queue、世代不一致等）
- 境界値: 5ケース（max_batch=0、容量上限等）
- **カバレッジ**: ライン・ブランチともに100%

**参照**: `ready_queue_coordinator.rs`, `ready_queue_coordinator/tests.rs`

---

### Q3-2. ミドルウェアはどう実装する？

**B3-2.**

`MiddlewareChain` トレイトを実装し、CompositeMiddlewareで合成します（Phase 2B）：

**トレイト定義**:
```rust
pub trait MiddlewareChain {
  /// 実行前処理（順方向）
  fn before_invoke(&mut self, ctx: &InvokeContext) -> ControlFlow<(), ()>;

  /// 実行後処理（逆方向）
  fn after_invoke(&mut self, ctx: &InvokeContext, result: &InvokeResult);
}

pub struct InvokeContext {
  pub idx: MailboxIndex,
  pub throughput_hint: usize,
  pub start_time: Instant,
  pub metadata: HashMap<String, Value>,
}
```

**カスタムミドルウェア例**:
```rust
pub struct RateLimitMiddleware {
  token_bucket: TokenBucket,
}

impl MiddlewareChain for RateLimitMiddleware {
  fn before_invoke(&mut self, ctx: &InvokeContext) -> ControlFlow<(), ()> {
    match self.token_bucket.try_acquire() {
      Ok(_) => ControlFlow::Continue(()),
      Err(next_available_at) => {
        // トークンがない → 処理を保留
        ctx.metadata.insert("rate_limit_next", next_available_at);
        ControlFlow::Break(())
      }
    }
  }

  fn after_invoke(&mut self, ctx: &InvokeContext, result: &InvokeResult) {
    // 使用したトークン数をメトリクスに記録
    metrics::counter!("rate_limit.tokens_consumed", 1);
  }
}
```

**CompositeMiddleware**:
```rust
pub struct CompositeMiddleware {
  middlewares: Vec<Box<dyn MiddlewareChain>>,
}

impl MiddlewareChain for CompositeMiddleware {
  fn before_invoke(&mut self, ctx: &InvokeContext) -> ControlFlow<(), ()> {
    // 順方向実行
    for mw in &mut self.middlewares {
      match mw.before_invoke(ctx) {
        ControlFlow::Continue(_) => continue,
        ControlFlow::Break(_) => return ControlFlow::Break(()),
      }
    }
    ControlFlow::Continue(())
  }

  fn after_invoke(&mut self, ctx: &InvokeContext, result: &InvokeResult) {
    // 逆方向実行（リソース解放順序の制御）
    for mw in self.middlewares.iter_mut().rev() {
      mw.after_invoke(ctx, result);
    }
  }
}
```

**使用例**:
```rust
let middleware = CompositeMiddleware::new(vec![
  Box::new(TelemetryMiddleware::new()),
  Box::new(LoggingMiddleware::new()),
  Box::new(RateLimitMiddleware::new(100, Duration::from_secs(1))),
]);

let invoker = MessageInvoker::new(idx, registry)
  .with_middleware(middleware);
```

**参照**: `scheduler_sequences.puml` (middleware flow), `actor_scheduler_refactor.md` Section 4.4.1

---

### Q3-3. テスト戦略は？

**B3-3.**

各フェーズでテスト要求が定義されています：

**Phase 0**:
- ReadyQueueCoordinatorの単体テスト（13ケース）
- 全テストパス（既存テストに影響なし）

**Phase 1**:
- 単体テスト: 20ケース以上（正常8/異常7/境界5）
- カバレッジ: ライン・ブランチともに100%
- 統合テスト: 5シナリオ
  1. 単一アクター
  2. 100アクター並列 10kメッセージ
  3. 1000アクタースパイク
  4. Suspend/Resume連続
  5. 異常終了→再起動
- パフォーマンス: レイテンシ劣化<5%、スループット≥95%、メモリ<10%増

**Phase 2A**:
- ランタイム別統合テスト: 15ケース（Tokio/Embassy/Test各5）
- 10,000 msg/sec × 100アクター統合テスト

**Phase 2B**:
- ミドルウェア: 7ケース
- Guardian連携: 5ケース
- バックプレッシャ: 5ケース
- Suspend/Resume統合テスト

**Phase 3**:
- no_std統合テスト: 3アクター × 100メッセージ（QEMU + Embassy）
- Observability Hub統合テスト: 10ケース
- メトリクスがロックフリーであることをベンチマークで確認

**テストファイル配置規約**:
```
modules/actor-core/src/api/actor_scheduler/
  ready_queue_coordinator.rs         # 実装
  ready_queue_coordinator/
    tests.rs                          # 単体テスト

tests/
  integration/
    scheduler_phase1_basic.rs         # 統合テスト（Phase 1）
    scheduler_phase2_runtime.rs       # 統合テスト（Phase 2）
```

**参照**: `actor_scheduler_refactor.md` Section 5.1, `CLAUDE.md` テスト規約

---

## D1: 開発/デバッグ

### Q4-1. パフォーマンスベンチマークはどう実行する？

**D4-1.**

Criterionベースのベンチマークを使用します：

**ベースライン取得（Phase 0）**:
```bash
# 現行実装のベースライン
cargo bench --bench mailbox_throughput > benchmarks/baseline_phase0.txt
cargo bench --bench scheduler_latency >> benchmarks/baseline_phase0.txt
cargo bench --bench ready_queue_ops >> benchmarks/baseline_phase0.txt
```

**Phase 1以降の比較**:
```bash
# 新実装のベンチマーク
cargo bench --bench mailbox_throughput --features new-scheduler > benchmarks/phase1.txt
cargo bench --bench scheduler_latency --features new-scheduler >> benchmarks/phase1.txt

# 比較（自動化スクリプト）
python scripts/compare_benchmarks.py \
  benchmarks/baseline_phase0.txt \
  benchmarks/phase1.txt \
  --threshold 0.05  # 5%劣化でエラー
```

**測定指標**:

| 指標 | 測定方法 | 許容値（Phase 1） |
|------|---------|------------------|
| **レイテンシ** | p50/p95/p99（μs） | +5%以内 |
| **スループット** | messages/sec | ≥95% |
| **CPU使用率** | `perf stat` | +10%以内 |
| **メモリ** | `valgrind --tool=massif` | +10%以内 |

**ベンチマークシナリオ**:
```rust
// benches/mailbox_throughput.rs
fn bench_1_actor_100k_msgs(c: &mut Criterion) {
  c.bench_function("1_actor_100k", |b| {
    b.iter(|| {
      // 1 actor × 100,000 messages
    });
  });
}

fn bench_100_actors_10k_msgs(c: &mut Criterion) {
  c.bench_function("100_actors_10k", |b| {
    b.iter(|| {
      // 100 actors × 10,000 messages
    });
  });
}
```

**CI自動化（Phase 1）**:
```yaml
# .github/workflows/benchmarks.yml
name: Benchmarks
on:
  schedule:
    - cron: '0 2 * * *'  # 夜間実行

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - run: cargo bench --features new-scheduler > current.txt
      - run: python scripts/compare_benchmarks.py baseline.txt current.txt --threshold 0.05
      - if: failure()
        uses: slackapi/slack-github-action@v1
        with:
          payload: '{"text": "Benchmark regression detected!"}'
```

**参照**: `actor_scheduler_refactor.md` Section 5.2, `benchmarks/README.md`

---

### Q4-2. ロールバック手順は？

**D4-2.**

Feature flag により即座にロールバック可能です：

**Phase 0-3でのロールバック**:

1. **Cargo.toml で feature を無効化**:
```toml
[features]
default = []  # "new-scheduler" を削除
new-scheduler = ["std"]
```

2. **再ビルドとテスト**:
```bash
cargo clean
cargo build --release
cargo test --workspace
```

3. **デプロイ**:
```bash
# 旧実装でデプロイ
cargo build --release
./deploy.sh
```

**Phase 4（feature flag削除後）のロールバック**:

1. **ロールバック用タグを作成しておく**:
```bash
# Phase 3完了時
git tag -a phase3-stable -m "Stable before Phase 4"
git push origin phase3-stable
```

2. **ロールバック実行**:
```bash
git checkout phase3-stable
cargo build --release
./deploy.sh
```

**ロールバック手順書テンプレート**:

`docs/migration/scheduler_refactor_rollback.md`に以下を記載：

- ロールバック判断基準（レイテンシ+20%以上、致命的バグ等）
- ロールバックコマンド（feature flag切り替え）
- 検証手順（テスト実行、smoke test）
- 監視項目（メトリクス、ログ）
- エスカレーション手順

**参照**: `actor_scheduler_refactor.md` Section 5.3

---

### Q4-3. トラブルシュート：ready queueが詰まったら？

**D4-3.**

以下の手順で診断します：

**Step 1: メトリクス確認**
```rust
// ObservabilityHub 経由で以下を監視
metrics::gauge!("ready_queue.size", queue_size);
metrics::gauge!("ready_queue.pending_duration_ms", pending_duration);
metrics::counter!("ready_queue.register_count");
metrics::counter!("ready_queue.drain_count");
```

**症状別診断**:

| 症状 | 原因候補 | 確認方法 |
|------|---------|---------|
| queue_size 増加し続ける | Workerが処理できない | `worker.busy_ratio` を確認 |
| register_count >> drain_count | drain_ready_cycleが呼ばれていない | Worker stacktraceを確認 |
| pending_duration_ms が長い | poll_wait_signalがブロック | シグナルチャネルの状態確認 |

**Step 2: デバッグログ有効化**
```rust
#[cfg(feature = "debug-scheduler")]
impl ReadyQueueCoordinator {
  fn register_ready(&mut self, idx: MailboxIndex) {
    tracing::debug!("register_ready: idx={:?}, queue_size={}", idx, self.queue_size());
    // ...
  }
}
```

**Step 3: トレーシング**
```bash
# OpenTelemetry traceを収集
RUST_LOG=cellex_actor_core::ready_queue_coordinator=trace cargo run

# Jaeger UIで以下を確認
# - register_ready → drain_ready_cycle のレイテンシ
# - poll_wait_signal の待機時間
# - invoke_batch の実行時間
```

**Step 4: 緊急対処**

```rust
// 緊急時: ready queue を強制フラッシュ
#[cfg(feature = "emergency-flush")]
pub fn emergency_flush_ready_queue(&mut self) {
  let mut state = self.state.lock().unwrap();
  tracing::warn!("Emergency flush: dropping {} indices", state.queue.len());
  state.queue.clear();
  state.queued.clear();
}
```

**既知の問題と対処**（Phase 2B以降で改善）:

1. **ロック競合**: `Mutex<QueueState>` → `DashSet`へ移行（Phase 2）
2. **シグナル遅延**: `signal_pending` フラグ → MPSCチャネルへ移行（Phase 2）
3. **Worker不足**: 固定Worker数 → 動的調整（Phase 4）

**参照**: `actor_scheduler_refactor.md` Section 7 (オープン課題)

---

### Q4-4. メモリリークを疑ったら？

**D4-4.**

以下の手順で調査します：

**Step 1: Valgrind Massif**
```bash
valgrind --tool=massif --massif-out-file=massif.out \
  cargo test --release ready_queue_coordinator

ms_print massif.out > massif_report.txt
```

**Step 2: jemalloc統計**
```toml
# Cargo.toml
[dependencies]
jemallocator = "0.5"

[profile.release]
debug = true  # シンボル情報を保持
```

```rust
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[cfg(test)]
mod leak_tests {
  #[test]
  fn test_no_leak_after_1000_cycles() {
    let stats_before = jemalloc_ctl::stats::allocated::read().unwrap();

    for _ in 0..1000 {
      // Suspend → Resume サイクル
    }

    let stats_after = jemalloc_ctl::stats::allocated::read().unwrap();
    assert!(stats_after - stats_before < 1024 * 1024); // <1MB増加
  }
}
```

**Step 3: 疑わしい箇所**

| コンポーネント | リーク候補 | 確認方法 |
|----------------|-----------|---------|
| **ReadyQueueCoordinator** | VecDequeが膨張 | `queue.len()` を定期ログ |
| **MailboxRegistry** | DashMapエントリが削除されない | `registry.len()` を監視 |
| **ActorCell** | Arc循環参照 | `Arc::strong_count()` を確認 |
| **Middleware** | メタデータHashMapが膨張 | `metadata.len()` を監視 |

**Step 4: loomテスト（並行性バグ検出）**
```rust
#[cfg(loom)]
mod loom_tests {
  use loom::sync::Arc;
  use loom::thread;

  #[test]
  fn test_concurrent_register_no_leak() {
    loom::model(|| {
      let coordinator = Arc::new(Mutex::new(Coordinator::new()));

      let handles: Vec<_> = (0..2).map(|i| {
        let c = coordinator.clone();
        thread::spawn(move || {
          c.lock().unwrap().register_ready(MailboxIndex::new(i, 0));
        })
      }).collect();

      for h in handles {
        h.join().unwrap();
      }

      // loomが全実行パスでリークがないことを確認
    });
  }
}
```

**参照**: `actor_scheduler_refactor.md` Section 5.1 (検証基準)

---

## 付録

### A. 用語集

| 用語 | 説明 |
|------|------|
| **ReadyQueueCoordinator** | Ready queueの登録/除外/排出を管理するコンポーネント |
| **WorkerExecutor** | ワーカタスクのライフサイクル管理と駆動を担当 |
| **MessageInvoker** | メッセージ実行ロジックとミドルウェアチェインを提供 |
| **MailboxIndex** | Mailboxを識別するインデックス（slot + generation） |
| **InvokeResult** | メッセージ実行結果（Completed/Yielded/Suspended/Failed/Stopped） |
| **Generational Index** | 世代番号付きインデックスによるuse-after-free防止機構 |
| **QueueState** | ready queueの内部状態（VecDeque + HashSet） |
| **MiddlewareChain** | メッセージ実行の前後処理を提供するトレイト |
| **ObservabilityHub** | メトリクス・トレーシング・ロギングの統合ハブ |

### B. 関連ADR

- [ADR-001: 命名ポリシー](../adr/2025-10-22-phase0-naming-policy.md) - Coordinator/Executor/Invoker命名の根拠
- [ADR-002: Suspend/Resume 責務配置](../adr/2025-10-22-phase0-suspend-resume-responsibility.md) - Suspend/Resumeの責務分担

### C. チェックリスト

**実装前**:
- [ ] 設計ドキュメント（`actor_scheduler_refactor.md`）を読んだ
- [ ] 関連ADRを読んだ
- [ ] PlantUMLダイアグラムで責務を理解した
- [ ] 既存の`DefaultReadyQueueCoordinator`実装を読んだ

**実装中**:
- [ ] テストを先に書いた（TDD）
- [ ] ドキュメントコメントを書いた
- [ ] `clippy`と`rustfmt`を実行した
- [ ] 単体テストが全てパスした

**実装後**:
- [ ] 統合テストを追加した
- [ ] ベンチマークを実行して回帰がないことを確認した
- [ ] PRを作成してレビュー依頼した
- [ ] ADRに実装の判断を記録した（必要に応じて）

---

**このドキュメントに追加してほしいQ&Aがあれば、Issueまたは PRで提案してください。**

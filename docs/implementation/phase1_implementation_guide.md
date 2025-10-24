# Phase 1 実装ガイド - ReadyQueueCoordinator

## 概要

このドキュメントは、ActorSchedulerリファクタリングのPhase 1において、ReadyQueueCoordinatorを実装するための詳細なガイドです。Phase 0で作成したプロトタイプを基に、プロダクション品質の実装を目指します。

**対象フェーズ**: Phase 1（約2週間）
**前提条件**: Phase 0完了（プロトタイプとテストが動作している）
**完了基準**: 全テスト通過、ベンチマーク目標達成、コードレビュー承認

---

## 1. Phase 1 の目標

### 1.1 機能目標

- [ ] ReadyQueueCoordinatorの完全実装（DashSet版）
- [ ] 既存テストとの互換性維持
- [ ] 新規単体テスト追加（20ケース以上）
- [ ] 統合テスト追加（5シナリオ）
- [ ] ベンチマーク実行とベースライン比較

### 1.2 品質目標

| 指標 | 目標値 | 測定方法 |
|------|--------|---------|
| **レイテンシ劣化** | < 5% | `cargo bench --bench scheduler_latency` |
| **スループット** | ≥ 95% | `cargo bench --bench mailbox_throughput` |
| **メモリオーバーヘッド** | < 10% | `valgrind --tool=massif` |
| **テストカバレッジ** | 100% | ライン・ブランチともに |
| **統合テスト完了時間** | < 30秒 | 各シナリオ |

---

## 2. 実装タスク

### 2.1 Week 1: コア実装と単体テスト

#### タスク1-1: DashSetベースのQueueState実装

**ファイル**: `modules/actor-core/src/api/actor_scheduler/ready_queue_coordinator.rs`

**変更内容**:

```rust
// Before (Phase 0 プロトタイプ)
struct CoordinatorState {
  queue:          VecDeque<MailboxIndex>,
  queued:         HashSet<MailboxIndex>,  // ← Mutex保護
  signal_pending: bool,
}

// After (Phase 1 本実装)
use dashmap::DashSet;

struct CoordinatorState {
  queue:          Arc<SegQueue<MailboxIndex>>,  // lock-free queue
  queued:         Arc<DashSet<MailboxIndex>>,   // lock-free set
  signal_channel: (Sender<()>, Receiver<()>),   // MPSC channel
}
```

**実装手順**:

1. `dashmap`と`crossbeam-queue`を依存関係に追加
   ```toml
   # Cargo.toml
   [dependencies]
   dashmap = "6.0"
   crossbeam-queue = "0.3"
   ```

2. `CoordinatorState`を新しい構造に変更

3. `register_ready`をlock-freeに変更
   ```rust
   fn register_ready(&mut self, idx: MailboxIndex) {
     // DashSetで重複チェック（lock-free）
     if self.state.queued.insert(idx) {
       self.state.queue.push(idx);  // SegQueueにpush（lock-free）
       let _ = self.state.signal_channel.0.send(());  // シグナル送信
     }
   }
   ```

4. `drain_ready_cycle`を実装
   ```rust
   fn drain_ready_cycle(&mut self, max_batch: usize, out: &mut Vec<MailboxIndex>) {
     out.clear();

     for _ in 0..max_batch {
       if let Some(idx) = self.state.queue.pop() {
         // DashSetでチェック（unregisterされた可能性があるため）
         if self.state.queued.remove(&idx).is_some() {
           out.push(idx);
         }
       } else {
         break;
       }
     }
   }
   ```

5. `poll_wait_signal`をチャネル経由に変更
   ```rust
   fn poll_wait_signal(&mut self, cx: &mut Context<'_>) -> Poll<()> {
     match self.state.signal_channel.1.try_recv() {
       Ok(_) => Poll::Ready(()),
       Err(TryRecvError::Empty) => {
         // Waker登録（実装は後述）
         Poll::Pending
       }
       Err(TryRecvError::Disconnected) => Poll::Ready(()),
     }
   }
   ```

**チェックポイント**:
- [ ] `cargo build` が通る
- [ ] 既存の単体テスト（13ケース）が通る

---

#### タスク1-2: 新規単体テスト追加

**ファイル**: `modules/actor-core/src/api/actor_scheduler/ready_queue_coordinator/tests.rs`

**追加テストケース**:

```rust
// 並行性テスト（7ケース）
#[test]
fn test_concurrent_register_from_multiple_threads() {
  // 10スレッドから同時に register_ready を呼び出し
  // 重複なく全て登録されることを確認
}

#[test]
fn test_concurrent_drain_and_register() {
  // drainとregisterを並行実行
  // データ競合がないことを確認
}

#[test]
fn test_concurrent_unregister_during_drain() {
  // drain中にunregisterが呼ばれた場合
  // 正しくスキップされることを確認
}

// 境界値テスト（5ケース追加）
#[test]
fn test_drain_with_zero_batch() {
  // max_batch = 0 の場合、何も取り出されないことを確認
}

#[test]
fn test_register_max_u32_indices() {
  // MailboxIndex { slot: u32::MAX, generation: 0 } の登録
}

// パフォーマンステスト（2ケース）
#[test]
fn bench_register_ready_throughput() {
  // 1,000,000回のregister_readyを実行
  // < 1秒で完了することを確認（目安）
}

#[test]
fn bench_concurrent_register_10_threads() {
  // 10スレッドから各100,000回のregister_ready
  // スケーラビリティを確認
}
```

**チェックポイント**:
- [ ] 単体テスト総数 ≥ 20ケース
- [ ] 全テスト通過
- [ ] カバレッジ 100%（`cargo tarpaulin`で確認）

---

### 2.2 Week 2: 統合テストとベンチマーク

#### タスク2-1: 統合テスト実装

**ファイル**: `tests/integration/scheduler_phase1_basic.rs`（新規作成）

**シナリオ1: 単一アクター**
```rust
#[tokio::test]
async fn test_single_actor_1000_messages() {
  let system = ActorSystem::new();
  let actor = system.spawn(TestActor::new()).await;

  // 1,000メッセージを送信
  for i in 0..1000 {
    actor.send(TestMessage { id: i }).await;
  }

  // 全メッセージが処理されることを確認
  tokio::time::timeout(Duration::from_secs(5), async {
    while actor.processed_count() < 1000 {
      tokio::time::sleep(Duration::from_millis(10)).await;
    }
  })
  .await
  .expect("timeout");

  assert_eq!(actor.processed_count(), 1000);
}
```

**シナリオ2: 100アクター並列 10kメッセージ**
```rust
#[tokio::test]
async fn test_100_actors_10k_messages() {
  let system = ActorSystem::new();
  let mut actors = Vec::new();

  // 100アクター生成
  for _ in 0..100 {
    actors.push(system.spawn(TestActor::new()).await);
  }

  // 各アクターに100メッセージ送信（合計10,000メッセージ）
  for actor in &actors {
    for i in 0..100 {
      actor.send(TestMessage { id: i }).await;
    }
  }

  // 全アクターが全メッセージを処理することを確認
  tokio::time::timeout(Duration::from_secs(30), async {
    for actor in &actors {
      while actor.processed_count() < 100 {
        tokio::time::sleep(Duration::from_millis(10)).await;
      }
    }
  })
  .await
  .expect("timeout");

  for actor in &actors {
    assert_eq!(actor.processed_count(), 100);
  }
}
```

**シナリオ3: 1000アクタースパイク**
```rust
#[tokio::test]
async fn test_1000_actors_spike_load() {
  let system = ActorSystem::new();

  // 1,000アクターを一気に生成
  let actors: Vec<_> = (0..1000)
    .map(|_| system.spawn(TestActor::new()))
    .collect::<Vec<_>>();

  // 各アクターに1メッセージ送信
  for actor in &actors {
    actor.send(TestMessage { id: 0 }).await;
  }

  // 全アクターが処理完了
  tokio::time::timeout(Duration::from_secs(30), async {
    for actor in &actors {
      while actor.processed_count() < 1 {
        tokio::time::sleep(Duration::from_millis(10)).await;
      }
    }
  })
  .await
  .expect("timeout");
}
```

**シナリオ4: Suspend/Resume連続**
```rust
#[tokio::test]
async fn test_suspend_resume_cycle() {
  let system = ActorSystem::new();
  let actor = system.spawn(SuspendableActor::new()).await;

  for _ in 0..10 {
    // Suspend
    actor.send(SuspendMessage).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    // メッセージ送信（処理されない）
    actor.send(TestMessage { id: 0 }).await;
    assert_eq!(actor.processed_count(), 0);

    // Resume
    actor.send(ResumeMessage).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    // メッセージが処理される
    assert_eq!(actor.processed_count(), 1);
    actor.reset_count();
  }
}
```

**シナリオ5: 異常終了→再起動**
```rust
#[tokio::test]
async fn test_actor_failure_restart() {
  let system = ActorSystem::new();
  let actor = system.spawn_with_supervisor(
    FailingActor::new(),
    SupervisorStrategy::OneForOne {
      max_restarts: 3,
      within:       Duration::from_secs(10),
    },
  ).await;

  // エラーを発生させる
  actor.send(FailMessage).await;
  tokio::time::sleep(Duration::from_millis(100)).await;

  // 再起動後も動作する
  actor.send(TestMessage { id: 0 }).await;
  tokio::time::sleep(Duration::from_millis(100)).await;

  assert_eq!(actor.processed_count(), 1);
}
```

**チェックポイント**:
- [ ] 全統合テストシナリオ通過
- [ ] 各シナリオ < 30秒で完了

---

#### タスク2-2: ベンチマーク実行と比較

**ファイル**: `benches/mailbox_throughput.rs`, `benches/scheduler_latency.rs`（既存）

**実行手順**:

1. **ベースライン測定**（Phase 0で取得済み）
   ```bash
   git checkout main
   cargo bench --bench mailbox_throughput > benchmarks/baseline_phase0.txt
   cargo bench --bench scheduler_latency >> benchmarks/baseline_phase0.txt
   ```

2. **Phase 1実装での測定**
   ```bash
   git checkout feature/phase1-coordinator
   cargo bench --bench mailbox_throughput > benchmarks/phase1.txt
   cargo bench --bench scheduler_latency >> benchmarks/phase1.txt
   ```

3. **比較スクリプト実行**（Week 2で実装）
   ```bash
   python scripts/compare_benchmarks.py \
     benchmarks/baseline_phase0.txt \
     benchmarks/phase1.txt \
     --threshold 0.05
   ```

   **期待出力**:
   ```
   Benchmark Comparison Report
   ============================

   mailbox_throughput/1_actor_100k:
     Baseline: 2,450,000 msgs/sec
     Phase 1:  2,380,000 msgs/sec
     Change:   -2.86% ✅ (< 5% threshold)

   scheduler_latency/p95:
     Baseline: 45.2 μs
     Phase 1:  46.8 μs
     Change:   +3.54% ✅ (< 5% threshold)

   Overall: PASS
   ```

4. **メモリプロファイリング**
   ```bash
   # Valgrind Massif
   valgrind --tool=massif --massif-out-file=massif_phase1.out \
     cargo test --release ready_queue_coordinator

   ms_print massif_phase1.out > benchmarks/memory_phase1.txt

   # Phase 0と比較
   diff benchmarks/memory_phase0.txt benchmarks/memory_phase1.txt
   ```

**チェックポイント**:
- [ ] レイテンシ劣化 < 5%
- [ ] スループット ≥ 95%
- [ ] メモリオーバーヘッド < 10%

---

## 3. テスト移行手順

### 3.1 既存テストの確認

Phase 0で実装したテストが全て通過することを確認：

```bash
cargo test -p cellex-actor-core-rs ready_queue_coordinator
```

**期待結果**: 13/13 tests passed

### 3.2 ReadyQueueStateテストの移植

既存の`ReadyQueueState`テストを`queue_state/tests.rs`に移植（Phase 1完了後）：

**Before**:
```
modules/actor-core/src/api/actor_scheduler/
  ready_queue_scheduler/
    ready_queue_state.rs      # 既存実装
    ready_queue_state_test.rs # 既存テスト
```

**After**:
```
modules/actor-core/src/api/actor_scheduler/
  ready_queue_coordinator/
    queue_state.rs            # ReadyQueueStateの新実装
    queue_state/
      tests.rs                # 移植したテスト
```

**移植手順**:

1. `ready_queue_state_test.rs`の全テストケースをコピー
2. `CoordinatorState`の新しいAPIに合わせて調整
3. テスト実行して全通過を確認

---

## 4. ワーカループ実装例

### 4.1 WorkerExecutorの骨格（Phase 2Aで実装予定）

Phase 1では実装しませんが、設計の参考として記載：

```rust
pub struct WorkerExecutor {
  coordinator: Arc<Mutex<dyn ReadyQueueCoordinator>>,
  throughput_hint: usize,
  max_batch: usize,
}

impl WorkerExecutor {
  pub async fn run(&self) {
    let mut batch = Vec::with_capacity(self.max_batch);

    loop {
      // 1. シグナル待機
      {
        let mut coordinator = self.coordinator.lock().unwrap();
        let mut cx = Context::from_waker(noop_waker_ref());

        match coordinator.poll_wait_signal(&mut cx) {
          Poll::Ready(_) => {},
          Poll::Pending => {
            // 非同期待機
            drop(coordinator);
            tokio::time::sleep(Duration::from_millis(1)).await;
            continue;
          }
        }
      }

      // 2. Ready queueから取り出し
      {
        let mut coordinator = self.coordinator.lock().unwrap();
        coordinator.drain_ready_cycle(self.max_batch, &mut batch);
      }

      // 3. 各インデックスに対して処理
      for idx in &batch {
        // Phase 2Bで実装: MessageInvoker::invoke_batch
        let result = self.invoke_batch(*idx, self.throughput_hint);

        // 4. 結果をCoordinatorに通知
        let mut coordinator = self.coordinator.lock().unwrap();
        coordinator.handle_invoke_result(*idx, result);
      }

      batch.clear();
    }
  }

  fn invoke_batch(&self, idx: MailboxIndex, throughput_hint: usize) -> InvokeResult {
    // Phase 2Bで実装
    todo!("MessageInvoker integration")
  }
}
```

---

## 5. コードレビューチェックリスト

### 5.1 実装者チェックリスト

Phase 1完了時に以下を確認してください：

- [ ] **コード品質**
  - [ ] `cargo clippy --workspace -- -D warnings` が通る
  - [ ] `cargo fmt --all` 適用済み
  - [ ] ドキュメントコメント完備（`#![deny(missing_docs)]`に準拠）
  - [ ] `makers ci-check -- dylint` が通る

- [ ] **テスト**
  - [ ] 単体テスト ≥ 20ケース、全通過
  - [ ] 統合テスト 5シナリオ、全通過
  - [ ] カバレッジ 100%（`cargo tarpaulin --out Html`で確認）

- [ ] **パフォーマンス**
  - [ ] ベンチマーク実行済み
  - [ ] レイテンシ劣化 < 5%
  - [ ] スループット ≥ 95%
  - [ ] メモリオーバーヘッド < 10%

- [ ] **ドキュメント**
  - [ ] 実装ドキュメント更新（このガイド含む）
  - [ ] ADR作成（必要に応じて）
  - [ ] CHANGELOGエントリ追加

### 5.2 レビュアーチェックリスト

- [ ] **アーキテクチャ**
  - [ ] ReadyQueueCoordinatorトレイトの責務が守られている
  - [ ] ステートレス設計が維持されている
  - [ ] lock-free実装が正しく使われている

- [ ] **並行性**
  - [ ] データ競合がない（loomテストで確認）
  - [ ] デッドロックの可能性がない
  - [ ] メモリオーダリングが適切

- [ ] **テスト**
  - [ ] エッジケースがカバーされている
  - [ ] 並行性テストが十分
  - [ ] パフォーマンステストが現実的

- [ ] **互換性**
  - [ ] 既存テストが全通過
  - [ ] 後方互換性が保たれている（API変更なし）

---

## 6. トラブルシューティング

### 6.1 よくある問題

#### 問題1: DashSetでデッドロック

**症状**: テスト実行中にハング

**原因**: DashSetのShardedLockで予期しないロック順序

**解決策**:
```rust
// Bad: ロック順序が不定
self.queued.insert(idx);
self.queue.push(idx);

// Good: アトミック操作で順序保証
if self.queued.insert(idx) {
  self.queue.push(idx);
}
```

#### 問題2: SegQueueからのpopが遅い

**症状**: drain_ready_cycleのパフォーマンスが悪い

**原因**: SegQueueはpopが遅い場合がある

**解決策**: crossbeam-channelのbounded channelを検討
```rust
use crossbeam_channel::bounded;

let (tx, rx) = bounded(1000);
// tx.send(idx) でキューイング
// rx.try_iter().take(max_batch) でdrain
```

#### 問題3: poll_wait_signalがPendingから戻らない

**症状**: Workerが起動しない

**原因**: Waker登録が正しくない

**解決策**: Tokioのnotify/notified APIを使用
```rust
use tokio::sync::Notify;

struct CoordinatorState {
  notify: Arc<Notify>,
  // ...
}

// register_ready時
self.notify.notify_one();

// poll_wait_signal時
async fn wait_for_signal(&self) {
  self.notify.notified().await;
}
```

---

## 7. 次のステップ（Phase 2A）

Phase 1完了後、Phase 2Aで以下を実装します：

- [ ] WorkerExecutor抽象の導入
- [ ] Tokio/Embassy/テスト向けランタイム実装
- [ ] ランタイム別統合テスト 15ケース

**準備事項**:
- Phase 1のベンチマーク結果をベースラインとして保存
- WorkerExecutorトレイト設計のドラフト作成（ADR）
- Embassy統合の調査（no_std対応）

---

## 8. 参照

- [actor_scheduler_refactor.md](../design/actor_scheduler_refactor.md) - 全体設計
- [scheduler_implementation_faq.md](../design/scheduler_implementation_faq.md) - 実装FAQ
- [ready_queue_coordinator.rs](../../modules/actor-core/src/api/actor_scheduler/ready_queue_coordinator.rs) - Phase 0プロトタイプ
- [ADR-001: 命名ポリシー](../adr/2025-10-22-phase0-naming-policy.md)
- [ADR-002: Suspend/Resume 責務配置](../adr/2025-10-22-phase0-suspend-resume-responsibility.md)

---

**最終更新**: 2025-10-22
**フェーズ**: Phase 0（ガイド作成）
**次回更新**: Phase 1 開始時

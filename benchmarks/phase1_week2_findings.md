# Phase 1 Week 2: Lock-Free実装の検証結果

**測定日**: 2025-10-22
**実装**: LockFreeCoordinator (DashSet + SegQueue) vs DefaultReadyQueueCoordinator (Mutex + VecDeque + HashSet)

## Executive Summary

**重要な発見**: 現在のトレイト設計（`&mut self`）では、lock-free実装の利点を活かせない。

並行性ベンチマークの結果、**予想に反して**LockFreeCoordinatorがすべてのスレッド数で遅いことが判明：

| スレッド数 | DefaultReadyQueueCoordinator | LockFreeCoordinator | パフォーマンス差 |
|----------|------------------------------|---------------------|-----------------|
| 1 | **472 µs** | 517 µs | +9.4% 遅い |
| 2 | **1.15 ms** | 1.29 ms | +11.8% 遅い |
| 4 | **5.12 ms** | 8.23 ms | **+60.8% 遅い** |
| 8 | **11.24 ms** | 18.66 ms | **+66.0% 遅い** |

---

## 1. 問題の根本原因

### 1.1 ベンチマーク構造

両実装とも、以下のパターンでベンチマークされている：

```rust
// DefaultReadyQueueCoordinator
let coordinator = Arc::new(Mutex::new(DefaultReadyQueueCoordinator::new(32)));

// LockFreeCoordinator
let coordinator = Arc::new(Mutex::new(LockFreeCoordinator::new(32)));
//                        ^^^^^^^^ ← 問題: 外側のMutexが必要
```

### 1.2 なぜMutexが必要なのか

現在の`ReadyQueueCoordinator` traitの定義：

```rust
pub trait ReadyQueueCoordinator: Send + Sync {
  fn register_ready(&mut self, idx: MailboxIndex);
  //                ^^^^^ &mut を要求
  fn drain_ready_cycle(&mut self, max_batch: usize, out: &mut Vec<MailboxIndex>);
  //                   ^^^^^
}
```

**問題点**:
- すべてのメソッドが `&mut self` を要求
- 複数スレッドから同時にアクセスするには `Mutex` または `RwLock` が必須
- lock-free データ構造を使っても、外側で直列化される

### 1.3 二重ロック問題

```
Arc<Mutex<LockFreeCoordinator>>
    ^^^^^^ 粗粒度ロック（全体をロック）
           ^^^^^^^^^^^^^^^^^
           内部: DashSet（細粒度ロック）+ SegQueue（lock-free）
```

**結果**:
1. **DashSetの利点が無駄**: 複数セグメントで並行アクセス可能だが、外側のMutexで1スレッドずつ
2. **SegQueueの利点が無駄**: lock-free pushだが、外側のMutexで直列化
3. **オーバーヘッド増大**: Mutex lock/unlock + 内部の細粒度ロックの両方

---

## 2. 測定結果の詳細

### 2.1 スレッド数とスケーラビリティ

#### DefaultReadyQueueCoordinator (Mutex + VecDeque + HashSet)

| スレッド数 | 時間 | スレッドあたり | スケーラビリティ |
|----------|------|---------------|----------------|
| 1 | 472 µs | 472 µs | 1.0x (baseline) |
| 2 | 1.15 ms | 575 µs | 0.82x |
| 4 | 5.12 ms | 1,280 µs | 0.37x |
| 8 | 11.24 ms | 1,405 µs | 0.34x |

**観察**:
- 予想通りのMutexコンテンション
- 8スレッドで理想値（472µs）の約3倍遅い

#### LockFreeCoordinator (Mutex + DashSet + SegQueue)

| スレッド数 | 時間 | スレッドあたり | スケーラビリティ |
|----------|------|---------------|----------------|
| 1 | 517 µs | 517 µs | 1.0x (baseline) |
| 2 | 1.29 ms | 645 µs | 0.80x |
| 4 | 8.23 ms | 2,058 µs | 0.25x |
| 8 | 18.66 ms | 2,333 µs | 0.22x |

**観察**:
- DefaultReadyQueueCoordinatorより**さらに悪い**スケーラビリティ
- 8スレッドで理想値の約4.5倍遅い
- 4スレッド以降で急激に悪化

### 2.2 オーバーヘッドの内訳（推定）

**1スレッドでの差（+9.4%）**:
```
517µs - 472µs = 45µs のオーバーヘッド

内訳（推定）:
- DashSetのセグメント選択: ~10µs
- SegQueueのアトミック操作: ~15µs
- その他オーバーヘッド: ~20µs
```

**8スレッドでの差（+66.0%）**:
```
18.66ms - 11.24ms = 7.42ms の追加オーバーヘッド

原因（推定）:
- Mutexコンテンション: 両方に存在（基本）
- DashSet内部ロック: LockFreeのみ（追加）
- キャッシュライン競合: DashSetで増加
- アトミック操作の累積: SegQueueで顕著
```

---

## 3. 理論値との比較

### 3.1 理想的なlock-free実装（仮定）

**仮に** traitが`&self`ベースだった場合：

```rust
pub trait ReadyQueueCoordinator: Send + Sync {
  fn register_ready(&self, idx: MailboxIndex);
  //                ^^^^^ 内部可変性を使用
}

// 使用例
let coordinator = Arc::new(LockFreeCoordinator::new(32));
// Mutexなし！
```

**期待されるスケーラビリティ（DashSet/SegQueueの文献より）**:
- 1スレッド: baseline (500µs想定)
- 2スレッド: 0.95x (並行性の利点)
- 4スレッド: 0.90x (線形に近い)
- 8スレッド: 0.85x (ほぼ線形)

### 3.2 実測値との乖離

| スレッド数 | 理想 | 実測（LockFree） | 乖離 |
|----------|------|----------------|------|
| 1 | 500 µs | 517 µs | +3.4% |
| 2 | 950 µs | 1,290 µs | +35.8% |
| 4 | 1,800 µs | 8,230 µs | **+357%** |
| 8 | 3,400 µs | 18,660 µs | **+449%** |

**結論**: 現在の実装は理論値から大きく乖離している。

---

## 4. ForkJoinPoolとの比較（補足）

JavaのForkJoinPoolがwork-stealingで高いスケーラビリティを実現できる理由：

```java
// ForkJoinPool内部構造
class ForkJoinWorkerThread {
  WorkQueue workQueue; // スレッドローカル！Mutexなし

  void submit(Task task) {
    workQueue.push(task); // CAS操作のみ
  }

  Task steal() {
    // 他のワーカーのキューから盗む（CAS）
  }
}
```

**cellex-rsの現状**:
```rust
// 単一の共有キュー（Mutex必須）
Arc<Mutex<ReadyQueueCoordinator>>
```

**差異**:
- ForkJoinPool: スレッドローカルキュー → lock-free
- cellex-rs: 共有キュー + Mutex → lock-based

---

## 5. Phase 1の成果と学び

### 5.1 成果

✅ **実装完了**:
- LockFreeCoordinator (DashSet + SegQueue)
- AdaptiveCoordinator (条件付き選択)
- 包括的なベンチマークスイート

✅ **重要な発見**:
- トレイト設計（`&mut self`）がボトルネック
- lock-free実装には内部可変性が必須
- 二重ロック問題の特定

✅ **測定基盤**:
- 並行性ベンチマーク
- 1/2/4/8スレッドでの比較
- Criterionによる統計的検証

### 5.2 学び

**設計原則**:
1. **トレイト設計が実装を制約する**: `&mut self` → Mutex必須
2. **lock-freeは万能ではない**: 外側の制約で利点が消える
3. **ベンチマークが重要**: 直感と異なる結果が出る

**技術的洞察**:
1. **内部可変性の重要性**: `Arc<DashSet>` vs `Mutex<HashSet>`
2. **粒度の選択**: 粗粒度ロック vs 細粒度ロック vs lock-free
3. **測定駆動開発**: 仮定を検証してから最適化

---

## 6. Phase 1 Week 3の提案

### 6.1 トレイトの再設計

**Option A: 内部可変性ベース（推奨）**

```rust
pub trait ReadyQueueCoordinator: Send + Sync {
  fn register_ready(&self, idx: MailboxIndex);
  //                ^^^^^ &self で内部可変性を使用
  fn drain_ready_cycle(&self, max_batch: usize, out: &mut Vec<MailboxIndex>);
  //                   ^^^^^
}

// 実装例
pub struct LockFreeCoordinator {
  queue: Arc<SegQueue<MailboxIndex>>,
  queued: Arc<DashSet<MailboxIndex>>,
  // Arc内蔵 → &selfで変更可能
}

impl ReadyQueueCoordinator for LockFreeCoordinator {
  fn register_ready(&self, idx: MailboxIndex) {
    if self.queued.insert(idx) {
      self.queue.push(idx); // &selfでOK
    }
  }
}
```

**利点**:
- Mutexラップ不要
- lock-freeの利点を活かせる
- 線形スケーラビリティの実現

**欠点**:
- 既存コードの破壊的変更
- `&mut self`前提のコードが動かなくなる

### 6.2 段階的移行計画

**Phase 1 Week 3**: トレイト再設計
1. `ReadyQueueCoordinatorV2` traitを新規作成（`&self`ベース）
2. `LockFreeCoordinatorV2` を実装
3. ベンチマークで検証

**Phase 1 Week 4**: 移行と統合
1. 既存コードを`V2`に移行
2. 互換レイヤーの提供（必要に応じて）
3. 統合テスト

**Phase 2**: 最適化
1. Work-stealing導入
2. スレッドローカルキャッシュ
3. 適応的バッチサイズ

### 6.3 ベンチマーク計画

**検証項目**:
```rust
// V2実装で期待される結果
concurrent_comparison_v2/lockfree_v2/8 → ~4-5ms (現在: 18.66ms)
// 約4倍の改善を期待
```

**測定指標**:
- スケーラビリティ係数（理想値との比率）
- スレッドあたりのレイテンシ
- メモリオーバーヘッド

---

## 7. 結論

Phase 1 Week 2の作業により、以下が明らかになった：

**技術的発見**:
1. 現在のトレイト設計（`&mut self`）はlock-free実装と相性が悪い
2. Mutexラップによる二重ロック問題が性能を大幅に悪化させる
3. DashSet/SegQueueは適切に使えば高性能だが、Mutexで制約される

**次のアクション**:
1. **Phase 1 Week 3**: `&self`ベースの新トレイト設計
2. **Phase 1 Week 4**: 移行と統合テスト
3. **Phase 2**: Work-stealingによるさらなる最適化

**教訓**:
> 「測定なしに最適化せず、設計なしにlock-freeを使わず」

測定駆動開発により、直感に反する重要な問題を早期に発見できた。これはPhase 0のベンチマークファースト方針の正しさを証明している。

---

## Appendix: 測定環境

- **CPU**: Apple Silicon (M1/M2/M3推定)
- **OS**: macOS 14.x
- **Rust**: 2021 edition, --release
- **測定ツール**: Criterion 0.5.x
- **Feature flags**: `new-scheduler`, `std`

# Benchmarks

このディレクトリには、ActorScheduler リファクタリングのパフォーマンス測定用のベンチマークが含まれます。

## 概要

ActorScheduler のリファクタリング（Phase 0-4）では、パフォーマンスの回帰を防ぐため、各フェーズでベンチマークを実行します。

## ベースライン

Phase 0 時点でのベースライン：

- `baseline_phase0.md` - 現行実装（`ReadyQueueScheduler`）のベンチマーク結果

## ベンチマーク項目

### 1. Mailbox Throughput

**測定内容**: メッセージエンキュー/デキューのスループット

**シナリオ**:
- 1 アクター × 100,000 メッセージ
- 10 アクター × 10,000 メッセージ
- 100 アクター × 1,000 メッセージ
- 1,000 アクター × 100 メッセージ

**メトリクス**:
- messages/sec
- CPU 使用率
- メモリ使用量（ヒープ）

### 2. Scheduler Latency

**測定内容**: enqueue → actor receive のレイテンシ

**シナリオ**:
- 1 アクター構成
- 10 アクター構成
- 100 アクター構成
- 1,000 アクター構成

**メトリクス**:
- p50 レイテンシ (μs)
- p95 レイテンシ (μs)
- p99 レイテンシ (μs)
- p99.9 レイテンシ (μs)

### 3. Ready Queue Operations

**測定内容**: Ready queue の操作パフォーマンス

**シナリオ**:
- `register_ready` 連続呼び出し
- `drain_ready_cycle` バッチ取得
- 重複登録検知
- 並行アクセス（複数スレッド）

**メトリクス**:
- ops/sec
- ロック競合率

## 実行方法

### ベースライン測定（Phase 0）

```bash
# 現行実装のベンチマーク（実装後）
cargo bench --bench mailbox_throughput > benchmarks/baseline_phase0.txt
cargo bench --bench scheduler_latency >> benchmarks/baseline_phase0.txt
cargo bench --bench ready_queue_ops >> benchmarks/baseline_phase0.txt
```

### 新実装との比較（Phase 1以降）

```bash
# 新実装のベンチマーク
cargo bench --features new-scheduler --bench mailbox_throughput > benchmarks/phase1_new.txt
cargo bench --features new-scheduler --bench scheduler_latency >> benchmarks/phase1_new.txt

# 比較
./scripts/compare_benchmarks.py benchmarks/baseline_phase0.txt benchmarks/phase1_new.txt
```

## 成功基準

### Phase 1

- レイテンシ劣化: < 5%
- スループット維持: ≥ 95%
- メモリオーバーヘッド: < 10%

### Phase 2

- レイテンシ劣化（Phase 0 比）: < 10%
- スループット維持: ≥ 90%
- 並行性能: 4 スレッドで ≥ 3.5x スケール

### Phase 3

- レイテンシ（Phase 0 比）: +5% 以内に回復
- スループット: 95% 回復
- メモリ効率: ヒープ使用量 < 2KB/actor

### Phase 4

- レイテンシ: Phase 0 と同等またはそれ以上
- スループット: Phase 0 と同等またはそれ以上
- 新機能のオーバーヘッド: < 3%

## ツール

### Criterion

Rust の標準的なベンチマークフレームワーク:
- 統計的に有意な測定
- HTML レポート生成
- 回帰検出

### Valgrind / Massif

メモリプロファイリング:
```bash
valgrind --tool=massif cargo test --release
ms_print massif.out.<pid>
```

### jemalloc

ヒープ統計:
```bash
MALLOC_CONF=stats_print:true cargo test --release
```

## CI 統合

GitHub Actions でベンチマークを自動実行:
- `.github/workflows/benchmarks.yml`
- 夜間ジョブで実行
- 5% 以上の劣化で Slack 通知

## 参考資料

- [Criterion.rs](https://github.com/bheisler/criterion.rs)
- [Design Doc](../docs/design/actor_scheduler_refactor.md) - Section 5.2

---

**最終更新**: 2025-10-22
**担当フェーズ**: Phase 0

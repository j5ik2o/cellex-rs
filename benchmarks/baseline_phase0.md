# Phase 0 Baseline - 現行実装ベンチマーク

## 概要

このドキュメントは、ActorScheduler リファクタリング開始時点（Phase 0）の現行実装のベンチマーク結果を記録します。

**測定日**: (未実施)
**コミットハッシュ**: (未測定)
**実装**: `ReadyQueueScheduler` (現行)

## ステータス

🚧 **Phase 0 段階では、ベンチマーク実装は未完了です。**

Phase 1 開始前に以下のベンチマークを実装し、ベースライン測定を実施する必要があります：

- [ ] `benches/mailbox_throughput.rs` の実装
- [ ] `benches/scheduler_latency.rs` の実装
- [ ] `benches/ready_queue_ops.rs` の実装
- [ ] ベースライン測定の実行
- [ ] 結果の記録

## 計画されているベンチマーク

### 1. Mailbox Throughput

```rust
// benches/mailbox_throughput.rs
//
// 測定項目:
// - 1 actor × 100k messages
// - 10 actors × 10k messages
// - 100 actors × 1k messages
// - 1000 actors × 100 messages
//
// メトリクス: messages/sec, CPU, heap
```

### 2. Scheduler Latency

```rust
// benches/scheduler_latency.rs
//
// 測定項目:
// - enqueue → receive latency
// - 1, 10, 100, 1000 actor configurations
//
// メトリクス: p50, p95, p99, p99.9 (μs)
```

### 3. Ready Queue Operations

```rust
// benches/ready_queue_ops.rs
//
// 測定項目:
// - register_ready throughput
// - drain_ready_cycle performance
// - duplicate detection overhead
// - concurrent access (multi-threaded)
//
// メトリクス: ops/sec, lock contention
```

## 実行コマンド

```bash
# ベンチマーク実装後
cargo bench --bench mailbox_throughput > benchmarks/baseline_phase0.txt
cargo bench --bench scheduler_latency >> benchmarks/baseline_phase0.txt
cargo bench --bench ready_queue_ops >> benchmarks/baseline_phase0.txt

# Valgrind でメモリプロファイリング
valgrind --tool=massif --massif-out-file=benchmarks/baseline_phase0_mem.txt \
  cargo test --release --lib

# 結果の整形
cat benchmarks/baseline_phase0.txt | tee benchmarks/baseline_phase0.md
```

## Phase 0 での作業内容

Phase 0 では以下を完了しました：

1. ✅ ベンチマーク計画の策定
2. ✅ `benchmarks/` ディレクトリ構造の作成
3. ✅ ベースラインドキュメントのテンプレート作成
4. ⏭️ 実際のベンチマーク実装（Phase 1 へ持ち越し）

## 次のステップ

Phase 1 開始前:
1. Criterion ベースのベンチマークスイートを実装
2. CI での自動ベンチマーク実行を設定
3. ベースライン測定を実施し、このドキュメントを更新
4. `scripts/compare_benchmarks.py` 比較スクリプトを作成

---

**最終更新**: 2025-10-22
**フェーズ**: Phase 0
**次回更新**: Phase 1 開始時

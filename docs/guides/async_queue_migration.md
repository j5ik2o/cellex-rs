# Async Queue Migration Guide

## 1. Overview

- 旧 Tokio ベースの MPSC/SPSC キュー実装から、新しい async queue API への移行手順を示す。
- 目的は、`AsyncQueue` ベースの API に統一しつつ、割り込み対応や WaitQueue の改善恩恵を受けること。

## 2. 背景

- 旧 API (`ArcMpscBoundedQueue`, `ArcStack` など) は Tokio 依存かつ擬似 async 実装だった。
- 新 API は `AsyncQueue<T, K, B, A>` を中心とし、バックエンド/ミューテックス/ポリシーを差し替え可能。
- `SpinAsyncMutexDefault` / `SpinAsyncMutexCritical` などの alias で、利用環境ごとにポリシー選択が容易。

## 3. 移行ステップ

1. **依存確認**
   - `cellex-utils-core-rs` / `cellex-utils-std-rs` の最新バージョンへ更新。
   - 組込みターゲットでは `interrupt-cortex-m` feature を確認。

2. **型置換**
   - 旧 Tokio queue のラッパ型を `AsyncMpscQueue`, `AsyncSpscQueue` へ置換。
   - std/Tokio 環境では `tokio_bounded_mpsc_backend` のビルダー（`make_tokio_mpsc_queue`）を利用。

3. **割り込みポリシー選択**
   - ホスト環境: `SpinAsyncMutexDefault` もしくは `TokioAsyncMutex`。
   - Cortex-M 等: `SpinAsyncMutexCritical` + `interrupt-cortex-m` feature を有効化。

4. **エラーハンドリング更新**
   - `len`, `capacity`, `close` 等は `Result` を返すため、`QueueError`／`StackError` の取り扱いを更新。

5. **テスト調整**
   - async テストでは `tokio::test`/`async-std::test` など適切なランタイムを選択。
   - 割り込みポリシー適用時は `QueueError::WouldBlock` の挙動を確認。
   - 例: `modules/actor-std/tests/async_queue_migration.rs` では Tokio ランタイム上での基本動作と `WouldBlock` ハンドリングを検証している。

## 4. サンプル

```rust
use cellex_utils_std_rs::v2::collections::async_queue::make_tokio_mpsc_queue;

let queue = make_tokio_mpsc_queue::<String>(1024);
let (producer, consumer) = queue.into_mpsc_pair();
producer.offer("hello".into()).await?;
let msg = consumer.poll().await?;
```

組込み環境 (Cortex-M + Embassy) の例:

```rust
use cellex_utils_core_rs::sync::async_mutex_like::SpinAsyncMutexCritical;
use cellex_utils_core_rs::v2::collections::queue::{
    AsyncQueue, backend::SyncAdapterQueueBackend, VecRingBackend, VecRingStorage
};

let storage = VecRingStorage::with_capacity(16);
let backend = VecRingBackend::new_with_storage(storage, OverflowPolicy::Block);
let shared = ArcShared::new(SpinAsyncMutexCritical::new(SyncAdapterQueueBackend::new(backend)));
let queue: AsyncSpscQueue<u8, _, _> = AsyncQueue::new_spsc(shared);
```

## 5. 互換 API との共存

- フェーズ6 までは旧 API を並行維持。`#[deprecated]` を付与する予定なので、早めの切替を推奨。

## 6. FAQ

- **Q:** `len()` が `Result` を返すのはなぜ？
  - **A:** 割り込み文脈ではロック自体が許されず、`QueueError::WouldBlock` を通知するため。

- **Q:** 旧 API の `ArcMpscBoundedQueue` はどう置き換える？
  - **A:** std/Tokio 環境なら `make_tokio_mpsc_queue`, 組込みなら `SpinAsyncMutexCritical + SyncAdapterQueueBackend` を利用。

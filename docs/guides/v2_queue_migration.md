# Queue/Stack v2 Migration Guide

この文書は `modules/utils-core` / `modules/utils-std` の旧 Queue/Stack API から v2
ハンドルへ移行するための早見表です。

## 推奨置き換え

| 旧 API                                    | 新 API (v2)                                                          |
|-------------------------------------------|----------------------------------------------------------------------|
| `ArcMpscBoundedQueue<T>`                  | `utils_std::v2::collections::queue::StdMpscQueue<T>`                  |
| `ArcMpscUnboundedQueue<T>`                | `StdMpscQueue<T>` (容量上限を `OverflowPolicy::Grow` で指定)         |
| `ArcStack<T>`                             | `utils_std::v2::collections::stack::StdVecStack<T>`                   |
| `Queue::offer` / `poll` (旧 facade)       | `Queue::offer` / `poll` (v2)                                         |
| `QueueHandle` / `QueueRw` などの旧トレイト | v2 の `MpscProducer` / `MpscConsumer` / `SpscProducer` / `SpscConsumer` |

v2 では `TypeKey` と capability トレイト（`MultiProducer` など）により、コンパイル時に
誤用を防ぐことができます。MPSC/SPSC 向けには以下のメソッドを利用してください。

```rust
let queue: Queue<MyMsg, MpscKey, _, _> = /* ... */;
let (producer, consumer) = queue.into_mpsc_handles();
producer.offer(msg)?;
let received = consumer.poll()?;
```

std 環境からは `StdMpscQueue` / `StdSpscQueue` / `StdVecStack` のコンストラクタを利用すると
`StdSyncMutex` ベースのバックエンドが自動的に組み立てられます。

```rust
let queue = utils_std::v2::collections::queue::make_std_mpsc_queue_drop_oldest(1024);
let (producer, consumer) = queue.into_mpsc_handles();
```

## 非推奨マーク

旧構造体（`ArcMpscBoundedQueue`, `ArcStack` 等）には `#[deprecated]` が付与されており、
コンパイラの警告で新 API への移行が促されます。テスト等で旧 API を使い続ける場合は
`#![allow(deprecated)]` で警告を抑制できます。

## 参考

- v2 Queue/Stack の設計: `docs/design/collections_queue_spec.md`
- v2 std アダプタ: `modules/utils-std/src/v2/collections`

# Design: implement-block-overflow-policy

## Overview

本設計では Option A（Mailbox を AsyncQueue ベースに刷新）を採用し、既存の `WaitQueue`/`SyncAdapterQueueBackend` に実装済みの非同期待機を Mailbox 経路で活用できるようにする。これにより `OverflowPolicy::Block` が真に「空きができるまで待機」する動作へ遷移する。

## Architecture

### High-Level View

```
QueueMailboxHandle/Producer
        │ (async)
        ▼
   QueueMailboxCore
        │ (async)
        ▼
  MailboxQueueBackend (trait async)
        │
        ▼
  UserMailboxQueue (AsyncQueue)
        │
        ▼
SyncAdapterQueueBackend<VecRingBackend<EntryShared<M>>>
        │
        ▼
   VecRingBackend<EntryShared<M>>
```

### Data Flow (offer)

```
QueueMailboxHandle::offer_async
    │ await
    ▼
QueueMailboxCore::try_send_mailbox_async
    │ await
    ▼
UserMailboxQueue::offer_async
    │ await
    ▼
AsyncQueue::offer_shared
    │
    ├─ queueがFull → prepare_producer_wait() で WaitQueue 登録
    │                 waiter.await (タスクはPending)
    └─ queueに空き → backend.offer() → notify_consumer_waiter()
```

`poll` についても同様に `AsyncQueue::poll_shared` を利用し、consumer 側で空きが生じると `notify_producer_waiter()` が呼ばれて待機中の producer が再開される。

## Component Design

### 1. MailboxQueueBackend (async_trait)

- `#[async_trait(?Send)]` を付与し、以下を `async fn` 化する。
  - `offer(&self, message: M) -> Result<OfferOutcome, QueueError<M>>`
  - `poll(&self) -> Result<QueuePollOutcome<M>, QueueError<M>>`
  - `close(&self) -> Result<Option<M>, QueueError<M>>`
- `len`/`capacity`/`set_metrics_sink` は同期のまま維持（共有状態は内部で `SpinAsyncMutex` 等に委譲）。
- 既存実装（ユーザー・システム両キュー、テストダブル）を全面更新。

### 2. UserMailboxQueue の AsyncQueue 化

- 内部構成:
  ```rust
  type EntryShared<M> = ArcShared<Mutex<Option<M>>>;
  type Backend<M> = VecRingBackend<EntryShared<M>>;
  type AsyncBackend<M> = SyncAdapterQueueBackend<EntryShared<M>, Backend<M>>;
  type QueueMutex<M> = SpinAsyncMutex<AsyncBackend<M>>;
  type Queue<M> = AsyncQueue<EntryShared<M>, type_keys::MpscKey, AsyncBackend<M>, QueueMutex<M>>;
  ```
- 既存の `VecRingBackend` をそのまま活用しつつ、`SyncAdapterQueueBackend` が `WaitQueue` を扱う。
- `offer`/`poll`/`close`/`set_metrics_sink` を `async fn` として実装し、`Queue` 上の非同期メソッドを await。
- メトリクス送出やエラーハンドリングは従来ロジックを async 版へ移植。

### 3. QueueMailboxCore / Handle / Producer の async 化

- `QueueMailboxCore`:
  - `try_send_mailbox`, `try_send`, `try_dequeue_mailbox`, `close` を `async fn` 化。
  - システムキューとの交互動作も async 化し、`await` を挟む。
- `QueueMailboxHandle`, `QueueMailboxProducer`:
  - 公開 API (`offer`, `poll`, `flush`, `stop` 等) を async 化。
  - 既存の同期呼び出し箇所（スケジューラ、テスト）を更新。
- `MailboxSignal` との相互作用:
  - 送信後の `signal.notify()` は従来どおり同期呼び出し（通知は軽量な flag/tokio::Notify であり async 化不要）。必要に応じて await 可能なパスを追加検討。

### 4. System Queue 互換性

- System Mailbox（`SystemMailboxLane` 実装）も `MailboxQueueBackend` を実装しているため async 化対象となる。
- System queue が同期的なままで十分な場合は `async fn` 内で即値を返す実装に置き換える。

## API Changes

- 代表的なシグネチャ例:
  ```rust
  #[async_trait(?Send)]
  pub trait MailboxQueueBackend<M>: Clone
  where
    M: Element,
  {
    async fn offer(&self, message: M) -> Result<OfferOutcome, QueueError<M>>;
    async fn poll(&self) -> Result<QueuePollOutcome<M>, QueueError<M>>;
    async fn close(&self) -> Result<Option<M>, QueueError<M>>;
    // ...
  }

  impl<M> UserMailboxQueue<M>
  where
    M: Element,
  {
    pub async fn offer_async(&self, message: M) -> Result<OfferOutcome, QueueError<M>> { /* ... */ }
  }
  ```

- 呼び出し元は `queue_mailbox.offer(message).await` のように書き換える。

## Validation Plan

### 単体テスト

1. `UserMailboxQueue` に対する async テスト
   - `offer` → `offer`（満杯） → `poll` の順で待機と再開を検証。
   - 複数 producer (`tokio::spawn`) で FIFO 順が守られること。
   - `close` 時に待機中の producer が `QueueError::Disconnected` を受け取ること。

2. `QueueMailboxCore` 経由の統合テスト
   - `QueueMailbox` を構築し、Block ポリシーで offer を 3 回実行。3 回目が `poll` まで Pending になることを確認。
   - System queue 併用ケースやメトリクス連携もカバー。

### リグレッション

- Drop/Grow ポリシーが従来どおり動作するテストを更新。
- `QueueMailboxHandle::poll()` などの利用箇所で await 漏れが無いか `clippy` とビルドで検証。
- `cargo test --workspace`、`cargo clippy --workspace --all-targets`、`cargo +nightly fmt`、`./scripts/ci-check.sh all`。

### クロスコンパイル

- `cargo check -p cellex-actor-core-rs --target thumbv6m-none-eabi`
- `cargo check -p cellex-actor-core-rs --target thumbv8m.main-none-eabi`

## Risks & Mitigations

| リスク | 説明 | 緩和策 |
|--------|------|--------|
| 大量の async 化による呼び出し側崩壊 | Mailbox を同期前提で呼び出している箇所が多い | 段階的書き換えのため `try_send_async` 等の Transitional API を用意。 |
| `async_trait` のオーバーヘッド | `Box::pin` コストが発生 | `?Send` を活かして `no_std` 互換の軽量実装に抑える。必要であればジェネリック associated types を将来検討。 |
| `ArcShared<SpinAsyncMutex<...>>` のデッドロック | 複数 await を跨ぐ保持でハングの可能性 | `AsyncQueue::offer_shared` のロジックを流用し、`await` 前に必ず guard を drop する。 |

## Migration Notes

- 呼び出し元の同期 API を廃止するため、`QueueMailbox` を利用するすべての箇所を async へ移行。テストは `#[tokio::test]` または `block_on` ヘルパを使用。
- 既存の同期ユーティリティが必要な場合は、短期的に `block_on` ラッパーを用意（内部で `block_on(async { .. })`）。

## Timeline (Rough)

1. Week 1: トレイト async 化 + `UserMailboxQueue` async 化
2. Week 2: `QueueMailboxCore` 系の async 化とテスト更新
3. Week 3: ドキュメント更新・最終整備（CI / openspec validate）

## Open Questions

1. System queue 側の API を完全に async 化するか、一部 synchronous wrapper を残すか？
2. `MailboxSignal` が同期呼び出しのままで十分か、`await` 可能な通知が必要か？
3. embeddable runtime（`utils-embedded`）で同じ async API をどう扱うか？feature gate や `no_std` 確認が必要。

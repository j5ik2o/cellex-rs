# Async Queue/Stack Specification

## 1. 目的
- `utils-core` で既存の Queue/Stack 抽象を再利用しつつ、Tokio などの async ランタイム上で自然に使える API を提供する。
- TypeKey / Capability を維持し、コンパイル時に誤用を防ぎながら `await` ベースの offer/poll を公開する。

## 2. 共有ラッパ層

- 既存の `ArcShared<T>` をそのまま利用し、`T` に `AsyncMutexLike<B>` を実装した型（例: `tokio::sync::Mutex<B>`・`AsyncStdMutex<B>`）を格納する。
- `ArcAsyncShared` や `AsyncSharedAccess` など新しいラッパ型は導入しない。同期 v2 と同じく共有抽象のみで完結し、層を増やさない。
- ロック取得は `AsyncMutexLike::lock_async`（仮称）経由で `await` する実装とし、`SharedError` の扱いは同期版と同様に `Poisoned` / `BorrowConflict` を返す。
- `AsyncMutexLike<T>` の最小契約は「`lock_async(&self) -> impl Future<Output = Result<Guard<'_, T>, SharedError>>` を提供し、`Guard<'_, T>` は `Deref<Target = T>` かつ `Send`、トレイト自体も `Send + Sync`」であることを明示する。
- 割り込み文脈で `lock_async` が呼ばれた場合は `SharedError::InterruptContext` を返す責務を負い、環境ごとに `InterruptContextPolicy`（例：`cortex_m::interrupt::active()`／`critical_section::is_active()`）で判定する。

## 3. Async Backend 層

同期版（`SyncQueueBackend` / `StackBackend`）と同じ責務を持つが、**専用の async トレイト**として定義する。`async fn` を直接公開し、実装では `async_trait` もしくは GAT + `impl Future` を用いて非同期処理を記述する。

```rust
pub trait AsyncQueueStorage<T> {
    fn capacity(&self) -> usize;
    unsafe fn read_unchecked(&self, idx: usize) -> *const T;
    unsafe fn write_unchecked(&mut self, idx: usize, val: T);
}

#[async_trait::async_trait]
pub trait AsyncQueueBackend<T>: Send + Sync {
    type Storage: AsyncQueueStorage<T> + Send;

    fn new(storage: Self::Storage, policy: OverflowPolicy) -> Self
    where
        Self: Sized;

    async fn offer(&self, item: T) -> Result<OfferOutcome, QueueError>;
    async fn poll(&self) -> Result<T, QueueError>;
    async fn close(&self);

    fn len(&self) -> usize;
    fn capacity(&self) -> usize;
}
```

※ 初期段階では `async_trait` マクロで実装し、将来的に GAT + `impl Future` へ移行する。

Stack 版も同様に `AsyncStackStorage` / `AsyncStackBackend` を定義し、`async fn push` / `async fn pop` を公開する。

> 備考: 当面は `async_trait` を採用し、将来 GAT + `impl Future` に移行する際もシグネチャ互換を保てるよう関連型を追加しない設計とする。

### 3.1 擬似 async アダプタ（SyncAdapterBackend）

- 初期フェーズでは同期 backend (`SyncQueueBackend`) をラップする `SyncAdapterBackend` を用意し、`async fn` 内で `WouldBlock → Poll::Pending` 変換や Waker 登録を行う。
- 真の async backend（`TokioBoundedMpscBackend` など）は `async fn offer/poll` を直接実装し、I/O 待機を `Waker` ベースで処理する。
- どちらの実装でも `OverflowPolicy::Block` を `await` 待機として扱い、割り込み文脈では `WouldBlock` を即時返却する。
- `SyncAdapterBackend` は `tokio::sync::Notify`（std 環境）や `futures_intrusive::channel::State` 等を内部で利用し、busy-wait ではなく Waker ドリブンで起床させる。

## 4. Async Queue ファサード

```rust
pub struct AsyncQueue<T, K, B, A = AsyncStdMutex<B>>
where
    K: TypeKey,
    B: AsyncQueueBackend<T>,
    A: AsyncMutexLike<B>;

impl<T, K, B, A> AsyncQueue<T, K, B, A>
where
    K: TypeKey,
    B: AsyncQueueBackend<T> + Send,
    A: AsyncMutexLike<B> + Send,
{
    pub async fn offer(&self, item: T) -> Result<OfferOutcome, QueueError>;
    pub async fn poll(&self) -> Result<T, QueueError>;
    pub async fn close(&self);
    pub async fn len(&self) -> usize;
    pub fn capacity(&self) -> usize;
}
```

- 内部では `ArcShared<A>` を保持し、`A::lock_async().await` で backend (`B`) にアクセスする。
- `A = AsyncStdMutex<B>` は std + Tokio ランタイム向けのデフォルトであり、thumb / no_std 環境では `CriticalSectionAsyncMutex<B>` などを明示的に指定する。
- `AsyncMpscProducer` / `AsyncMpscConsumer` / `AsyncSpscProducer` / `AsyncSpscConsumer` を Capability に基づいて追加し、同期版と同じ型制約（例: `MpscKey: MultiProducer + SingleConsumer`）を維持する。
- `PriorityKey` 用 async ラッパ (`peek_min`) も提供。
- `capacity()` は Backend 初期化時に確定する不変値をそのまま返すためロック不要で提供できる。ロックや再計算が必要な Backend では `async fn capacity` に拡張する。
- Future の Drop（キャンセル）時は待機ノードを `Drop` 実装で必ず除去し、`Notify`／Waker リストがリークしないようにする。`Pin<&mut Self>` を用いた自己参照 Future を避け、`Arc<WaitNode>` + `Weak` で安全に削除する方針。
- 利用者向けには `type TokioMpscQueue<T>` や `AsyncQueue::builder()` といったエイリアス／ビルダーを提供し、公開 API からジェネリクスの複雑さを隠蔽する。

## 5. Async Stack ファサード

```rust
pub struct AsyncStack<T, B, A = AsyncStdMutex<B>>
where
    B: AsyncStackBackend<T>,
    A: AsyncMutexLike<B>;

impl<T, B, A> AsyncStack<T, B, A>
where
    B: AsyncStackBackend<T> + Send,
    A: AsyncMutexLike<B> + Send,
{
    pub async fn push(&self, item: T) -> Result<PushOutcome, StackError>;
    pub async fn pop(&self) -> Result<T, StackError>;
    pub async fn peek(&self) -> Result<Option<T>, StackError>
    where
        T: Clone + Send;
}
```

- `StackOverflowPolicy` は同期版と同様に `Block` / `Grow` のみを提供し、LIFO の整合性を保つため `DropNewest` / `DropOldest` はサポートしない。

## 6. Tokio backend と擬似 async backend
- フェーズ1では `SyncAdapterBackend` が `SyncQueueBackend` を Future 化し、`WouldBlock` を `Poll::Pending` に変換する。
- フェーズ2以降で `TokioBoundedMpscBackend` / `TokioUnboundedMpscBackend` を `AsyncQueueBackend` として実装し、`Waker` 経由でバックプレッシャを制御する。
- `tokio::sync::Mutex` / `tokio::sync::RwLock` を `AsyncMutexLike` として再利用しつつ、std-only 構成（`AsyncStdMutex`）と no_std 構成（`CriticalSectionAsyncMutex` 等）を切り替えられるようにする。

## 7. エラーと SharedError
| SharedError        | QueueError     | StackError     | async 変換                         |
| ------------------ | -------------- | -------------- | --------------------------------- |
| `Poisoned`         | `Disconnected` | `Disconnected` | `Err(Disconnected)` のまま伝播       |
| `BorrowConflict`   | `WouldBlock`   | `WouldBlock`   | `Poll::Pending` に変換             |
| `InterruptContext` | `WouldBlock`   | `WouldBlock`   | ISR では `Err(WouldBlock)` を即時返却 |

## 8. API 例

```rust
let queue = AsyncStdMpscQueue::with_capacity(1024);
let (producer, consumer) = queue.into_mpsc_pair();

producer.offer(msg).await?;
let received = consumer.poll().await?;

// Note: 初期段階では同期バックエンドを Future でラップした「擬似 async」から開始し、
// Tokio などランタイム固有の真の async backend は後続フェーズで差し替え可能とする。
```

- `OverflowPolicy::Block` は async 環境では `await` での待機に相当し、ISR（`InterruptContext`）では同期版と同様に `QueueError::WouldBlock` を即時返却する。
- Future の Drop（キャンセル）時は待機リストから確実に除去し、後続タスクの `wake` が失われないようにする。

## 9. テスト方針
- `#[tokio::test]` で multi-producer / single-consumer の await シナリオを検証し、Capability 制約が型で維持されていることを確認する（UI テストを含む）。
- `OverflowPolicy::Block` の待機/解放、および Future の Drop（キャンセル）時に待機リストがリークしないことを async テストで確認する。
- thumb/no_std ターゲットでは async API をコンパイルのみ（Tokio が想定外のため）。
- 並行性の微妙な競合検出は実装が安定してから `loom` を用いた検証を検討する（現フェーズでは任意）。

## 10. ドキュメント
- `docs/guides/async_queue_migration.md` に旧 API からの移行手順を記載。
- API ドキュメントに async 版と同期版の対比表を掲載。

# Async Queue/Stack Specification

## 1. 目的
- `utils-core` で既存の Queue/Stack 抽象を再利用しつつ、Tokio などの async ランタイム上で自然に使える API を提供する。
- TypeKey / Capability を維持し、コンパイル時に誤用を防ぎながら `await` ベースの offer/poll を公開する。

## 2. 共有ラッパ層

- 既存の `ArcShared<T>` をそのまま利用し、`T` に `AsyncMutexLike<B>` を実装した型（例: `tokio::sync::Mutex<B>`・`AsyncStdMutex<B>`）を格納する。
- `ArcAsyncShared` や `AsyncSharedAccess` など新しいラッパ型は導入しない。同期 v2 と同じく共有抽象のみで完結し、層を増やさない。
- ロック取得は `AsyncMutexLike::lock` を `await` する実装とし、失敗時は `SharedError` を返す。
- **契約**: `lock` は `Future<Output = Result<Guard<'_, T>, SharedError>>` を返し、割り込み文脈では `Err(SharedError::InterruptContext)` を返す。`Guard` は `Deref<Target = T>` + `DerefMut` を実装し、`Send` な環境での共有に対応する。
- `AsyncMutexLike<T>` の最小契約は「`lock(&self) -> impl Future<Output = Result<Guard<'_, T>, SharedError>>` を提供し、`Guard<'_, T>` は `Deref<Target = T>` かつ `Send`、トレイト自体も `Send + Sync`」であることを明示する。
- 割り込み文脈で `lock` が呼ばれた場合は `SharedError::InterruptContext` を返す責務を負い、環境ごとに `InterruptContextPolicy`（例：Cortex-M では `SCB::vect_active()` を参照）で判定する。

### 2.1 InterruptContextPolicy の扱い

- `utils-core::sync::interrupt` モジュールに `InterruptContextPolicy` トレイトを導入する。
  ```rust
  pub trait InterruptContextPolicy {
    fn check_blocking_allowed() -> Result<(), SharedError>;
  }
  ```
- 各 `AsyncMutexLike` 実装はロック直前に `P::check_blocking_allowed()` を呼び出し、ブロックが許可されない場合は `Err(SharedError::InterruptContext)` を返す。ポリシー型 `P` は実装固有に保持し、`SpinAsyncMutex<P>` のように静的に決定する。
- `std`/Tokio 環境では `NeverInterruptPolicy`（常に許可）をデフォルトとし、`TokioAsyncMutex` などホスト向けミューテックスはこのポリシーを用いる。
- 組込み/no_std 環境では `CriticalSectionInterruptPolicy` や `CortexMInterruptPolicy` など、ターゲット固有の割り込み検査を行う実装を提供し、利用側が明示的に選択する。

## 3. Async Backend 層

同期版（`SyncQueueBackend` / `StackBackend`）と同じ責務を持つが、**専用の async トレイト**として定義する。`async fn` を直接公開し、実装では `async_trait` もしくは GAT + `impl Future` を用いて非同期処理を記述する。ただしストレージ層は新しいトレイトを設けず、同期版で既に利用している `QueueStorage<T>` / `StackStorage<T>` をそのまま共有する。これらのトレイトはポインタアクセスのみを扱うため `await` を伴う処理が発生せず、非同期用の別抽象を設けても利点がないためである。

```rust
#[async_trait::async_trait]
pub trait AsyncQueueBackend<T>: Send + Sync {
    type Storage: QueueStorage<T> + Send;

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

Stack 版も同様に `AsyncStackBackend` を定義し、関連ストレージは `StackStorage<T>` を再利用する。`async fn push` / `async fn pop` を公開しつつ、ストレージ層の契約は同期版と共通のものを維持する。

> 備考: 当面は `async_trait` を採用し、将来 GAT + `impl Future` に移行する際もシグネチャ互換を保てるよう関連型を追加しない設計とする。

### 3.1 擬似 async アダプタ（SyncAdapterBackend）

- 初期フェーズでは同期 backend (`SyncQueueBackend`) をラップする `SyncAdapterBackend` を用意し、`async fn` 内で `WouldBlock → Poll::Pending` 変換や Waker 登録を行う。
- 真の async backend（`TokioBoundedMpscBackend` など）は `async fn offer/poll` を直接実装し、I/O 待機を `Waker` ベースで処理する。
- どちらの実装でも `OverflowPolicy::Block` を `await` 待機として扱い、割り込み文脈では `WouldBlock` を即時返却する。
- `SyncAdapterBackend` は `tokio::sync::Notify`（std 環境）や `futures_intrusive::channel::State` 等を内部で利用し、busy-wait ではなく Waker ドリブンで起床させる。

## 4. Async Queue ファサード

```rust
pub struct AsyncQueue<T, K, B, A = SpinAsyncMutex<B>>
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
    pub async fn close(&self) -> Result<(), QueueError>;
    pub async fn len(&self) -> Result<usize, QueueError>;
    pub async fn capacity(&self) -> Result<usize, QueueError>;
    pub async fn is_empty(&self) -> Result<bool, QueueError>;
    pub async fn is_full(&self) -> Result<bool, QueueError>;
}
```

- 内部では `ArcShared<A>` を保持し、`A::lock().await?` の結果から backend (`B`) にアクセスする（`SharedError` は `QueueError::from` / `StackError::from` で写像して上位へ伝搬）。
- `A = SpinAsyncMutex<B>` を core クレートのデフォルトとし、no_std/組込み環境でも依存追加なしで利用できるようにする。`SpinAsyncMutexDefault<T>` / `SpinAsyncMutexCritical<T>` といった型エイリアスでポリシーを明示的に選択できるようにし、std/Tokio 環境での利用者向けには `utils-std` 側で `TokioAsyncMutex` や `AsyncStdMutex` を組み合わせた型エイリアス・ビルダーを提供する。
- `AsyncMpscProducer` / `AsyncMpscConsumer` / `AsyncSpscProducer` / `AsyncSpscConsumer` を Capability に基づいて追加し、同期版と同じ型制約（例: `MpscKey: MultiProducer + SingleConsumer`）を維持する。
- `PriorityKey` 用 async ラッパ (`peek_min`) も提供。
- `len` / `capacity` 系のクエリも `Result` を返し、割り込み文脈でロック取得がブロック不可と判断された場合には `QueueError::WouldBlock` を上位へ伝搬する。バックエンド側で非同期計算が必要になった際にも互換的に拡張できる。
- Future の Drop（キャンセル）時は待機ノードを `Drop` 実装で必ず除去し、`Notify`／Waker リストがリークしないようにする。`Pin<&mut Self>` を用いた自己参照 Future を避け、`Arc<WaitNode>` + `Weak` で安全に削除する方針。
- 利用者向けには `type TokioMpscQueue<T>` や `AsyncQueue::builder()` といったエイリアス／ビルダーを提供し、公開 API からジェネリクスの複雑さを隠蔽する。

## 5. Async Stack ファサード

```rust
pub struct AsyncStack<T, B, A = SpinAsyncMutex<B>>
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
    pub async fn close(&self) -> Result<(), StackError>;
    pub async fn len(&self) -> Result<usize, StackError>;
    pub async fn capacity(&self) -> Result<usize, StackError>;
}
```

- `len` / `capacity` / `is_empty` などのクエリ API も `Result` を返し、割り込み文脈では `StackError::WouldBlock` を通知する設計とする。
- `StackOverflowPolicy` は同期版と同様に `Block` / `Grow` のみを提供し、LIFO の整合性を保つため `DropNewest` / `DropOldest` はサポートしない。

## 6. Tokio backend と擬似 async backend
- フェーズ1では `SyncAdapterBackend` が `SyncQueueBackend` を Future 化し、`WouldBlock` を `Poll::Pending` に変換する。
- フェーズ2以降で `TokioBoundedMpscBackend` / `TokioUnboundedMpscBackend` を `AsyncQueueBackend` として実装し、`Waker` 経由でバックプレッシャを制御する。
- 現行実装では `modules/utils-std/src/v2/collections/async_queue/tokio_bounded_mpsc_backend.rs` に Tokio 版 backend を配置し、`async_queue.rs` から公開している。
- Embassy 版 backend (`modules/utils-embedded/src/v2/collections/async_queue.rs`) では `embassy_sync::channel::Channel` を用いて同様の待機キューを構成する。
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

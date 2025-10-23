# Queue/Stack Collection API Specification

## 1. 目的と適用範囲

本書は `modules/utils-core` を核とした Queue / Stack コレクション API の仕様を定義する。計画書（`collections_queue_refactor_plan.md`）で合意した設計方針を、実装に直結するインターフェイス・不変条件・エラー仕様として明文化する。

## 2. 型レベル区別子（TypeKey）と能力トレイト

### 2.1 TypeKey 定義

```rust
pub trait TypeKey: 'static {}

pub struct MpscKey;    impl TypeKey for MpscKey {}
pub struct SpscKey;    impl TypeKey for SpscKey {}
pub struct FifoKey;    impl TypeKey for FifoKey {}
pub struct PriorityKey;impl TypeKey for PriorityKey {}
```

### 2.2 能力トレイト

```rust
pub trait MultiProducer: TypeKey {}
pub trait SingleProducer: TypeKey {}
pub trait SingleConsumer: TypeKey {}
pub trait SupportsPeek: TypeKey {}

impl MultiProducer for MpscKey {}
impl SingleConsumer for MpscKey {}

impl SingleProducer for SpscKey {}
impl SingleConsumer for SpscKey {}

impl SingleProducer for FifoKey {}
impl SingleConsumer for FifoKey {}

impl SingleConsumer for PriorityKey {}
impl SupportsPeek for PriorityKey {}
impl SingleProducer for PriorityKey {}
```

### 2.3 契約対応表

| TypeKey       | Producer 能力 | Consumer 能力 | 追加能力       | 想定用途                          |
| ------------- | -------------- | -------------- | -------------- | --------------------------------- |
| `MpscKey`     | `MultiProducer`| `SingleConsumer` | —              | MPSC queue（複数 producer / 単一 consumer） |
| `SpscKey`     | `SingleProducer` | `SingleConsumer` | —            | SPSC / シングルスレッド queue              |
| `FifoKey`     | `SingleProducer` | `SingleConsumer` | —            | 固定長 FIFO（Ring）                     |
| `PriorityKey` | `SingleProducer`（初期実装） | `SingleConsumer` | `SupportsPeek` | 優先度付き queue（ヒープ等）                |

将来的に `MultiProducer` 拡張を検討する余地を残す。

Backend 実装は `where K: MultiProducer` のような制約を用いて型レベルで誤用を排除すること。

## 3. Storage 層

```rust
pub trait QueueStorage<T> {
    fn capacity(&self) -> usize;

    /// Safety: 呼び出し側はインデックスがストレージの不変条件内に収まることを保証する。
    unsafe fn read_unchecked(&self, idx: usize) -> *const T;
    unsafe fn write_unchecked(&mut self, idx: usize, val: T);
}
```

`unsafe` 操作は Storage 実装内に閉じ込め、Backend/Queue からは安全な API のみを提供すること。

## 4. Backend 層

### 4.1 OverflowPolicy と OfferOutcome

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverflowPolicy {
    DropNewest,
    DropOldest,
    Block,
    Grow,
}

impl Default for OverflowPolicy {
    fn default() -> Self { Self::DropOldest }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfferOutcome {
    Enqueued,
    DroppedOldest { count: usize },
    DroppedNewest { count: usize },
    GrewTo { capacity: usize },
}
```

### 4.2 QueueBackend トレイト

```rust
#[derive(Debug)]
pub enum QueueError {
    Full,
    Empty,
    Closed,
    Disconnected,
    WouldBlock,
    AllocError,
}

pub trait QueueBackend<T> {
    type Storage: QueueStorage<T>;

    fn new(storage: Self::Storage, policy: OverflowPolicy) -> Self;

    fn offer(&mut self, item: T) -> Result<OfferOutcome, QueueError>;
    fn poll(&mut self) -> Result<T, QueueError>;

    fn len(&self) -> usize;
    fn capacity(&self) -> usize;

    fn is_empty(&self) -> bool { self.len() == 0 }
    fn is_full(&self) -> bool { self.len() == self.capacity() }

    /// 一部実装でのみ使用。指定が無い場合は no-op。
    fn close(&mut self) {}
}
```

### 4.3 PriorityBackend

```rust
pub trait PriorityBackend<T: Ord>: QueueBackend<T> {
    fn peek_min(&self) -> Option<&T>;
}
```

Queue ファサードは `where B: PriorityBackend<T>` を満たす場合のみ `peek_min` などの API を公開する。

## 5. Shared 層（ArcShared）

```rust
#[derive(Debug)]
pub enum SharedError {
    Poisoned,
    BorrowConflict,
    InterruptContext,
}

pub struct ArcShared<T>(imp::ArcSharedImp<T>); // imp は cfg で切り替え

impl<T> ArcShared<T> {
    pub fn new(inner: T) -> Self {
        Self(imp::ArcSharedImp::new(inner))
    }

    pub fn try_with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> Result<R, SharedError> {
        self.0.try_with_mut(f)
    }

    pub fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        self.try_with_mut(f).expect("ArcShared::with_mut failed")
    }
}
```

### 5.1 SharedError → QueueError マッピング

| SharedError        | QueueError     | 意味                                     |
| ------------------ | -------------- | ---------------------------------------- |
| `Poisoned`         | `Disconnected` | 継続不能（Mutex 毒）                     |
| `BorrowConflict`   | `WouldBlock`   | 再試行で回復可能な借用衝突               |
| `InterruptContext` | `WouldBlock`   | 割り込み文脈などブロッキング不可の状況を示す |

Queue ファサード実装ではこの表に従い `SharedError` をマッピングすること。

### 5.2 Send / Sync / ISR 利用可否

| 実装（cfg）                   | `Send` / `Sync` 条件                        | ISR 内 `with_mut` | 備考                                        |
| ----------------------------- | ------------------------------------------ | ---------------- | ------------------------------------------- |
| `std`（`Arc<Mutex<T>>`）        | `Send + Sync`（`T: Send` の場合）             | 非推奨           | `loom` による競合検証対象                     |
| `std`（非アトミック → `Rc<RefCell<T>>`） | `Send`: 付与しない / `Sync`: 付与しない         | 非推奨           | 多スレッド非対応を型で表現                     |
| `embedded-cs`（critical-section） | `Send`: `T: Send` の場合のみ / `Sync`: 付与しない | 可（再入禁止、ネスト時は `BorrowConflict`） | `critical_section::with` をネスト禁止で利用     |
| `rp2040-sio` 等                | 実装依存。HAL 規約に合わせて `Send`/`Sync` を定義 | 可                | spinlock などターゲット固有実装に従う           |

## 6. Queue ファサード

```rust
pub struct Queue<T, K: TypeKey, B: QueueBackend<T>> {
    inner: ArcShared<B>,
    _pd: core::marker::PhantomData<(T, K)>,
}

impl<T, K: TypeKey, B: QueueBackend<T>> Queue<T, K, B> {
    pub fn new(shared_backend: ArcShared<B>) -> Self {
        Self { inner: shared_backend, _pd: core::marker::PhantomData }
    }

    pub fn offer(&self, item: T) -> Result<OfferOutcome, QueueError> {
        self.inner
            .try_with_mut(|backend| backend.offer(item))
            .map_err(|_| QueueError::Disconnected)?
    }

    pub fn poll(&self) -> Result<T, QueueError> {
        self.inner
            .try_with_mut(|backend| backend.poll())
            .map_err(|_| QueueError::Disconnected)?
    }
}

impl<T: Ord, B: PriorityBackend<T>> Queue<T, PriorityKey, B> {
    pub fn peek_min(&self) -> Result<Option<T>, QueueError> {
        self.inner
            .try_with_mut(|backend| Ok(backend.peek_min().cloned()))
            .map_err(|_| QueueError::Disconnected)?
    }
}

// 型エイリアスの例（利用者向け導線）
pub type MpscQueue<T, B> = Queue<T, MpscKey, B>;
pub type SpscQueue<T, B> = Queue<T, SpscKey, B>;

impl<T, B> Queue<T, MpscKey, B>
where
    B: QueueBackend<T>,
    MpscKey: MultiProducer + SingleConsumer,
{
    /// MPSC 専用ユーティリティをここに提供（例: producer ハンドル生成）。
}

impl<T, B> Queue<T, SpscKey, B>
where
    B: QueueBackend<T>,
    SpscKey: SingleProducer + SingleConsumer,
{
    // NOTE: TypeKey × Capability により、SPSC（SpscKey）では複数 Producer を生成するヘルパは公開しない。
    /// SPSC 専用ユーティリティを提供。複数 producer を生成する API は提供しない。
}

/// OfferOutcome をメトリクスに乗せる際の参考実装。
impl From<&OfferOutcome> for &'static str {
    fn from(outcome: &OfferOutcome) -> Self {
        match outcome {
            OfferOutcome::Enqueued => "enqueue",
            OfferOutcome::DroppedOldest { .. } => "drop_oldest",
            OfferOutcome::DroppedNewest { .. } => "drop_newest",
            OfferOutcome::GrewTo { .. } => "grow",
        }
    }
}
```

## 7. Stack 仕様

### 7.1 レイヤ責務（Queue と揃える）

**Stack は Drop 系ポリシー（`DropNewest` / `DropOldest`）を採用しない。**
LIFO の不変条件と整合せず、利用者にとって直感的でないためである（詳細は 7.2 を参照）。

| 層 | 代表トレイト / 型 | 責務 | 具体例 |
| --- | --- | --- | --- |
| Storage | `StackStorage<T>` | 生データバッファの読み書き（`alloc` のみ、`unsafe` はここに閉じ込める） | 固定長配列、リング風ストレージ |
| Backend | `StackBackend<T>` | `push/pop/peek` のロジック（常に `&mut self`、同期は担当しない） | 配列ベーススタック、動的拡張スタック |
| Shared | `ArcShared<T>` | 同期責務の吸収（std: `Arc<Mutex<_>>` / embedded: critical-section 等） | `ArcShared<ArrayStackBackend<T>>` |
| Stack API | `Stack<T, Backend>` | ユーザ API。Backend を委譲しエラー整合性・誤用防止を行う | `Stack<T, ArrayStackBackend<T>>` |

### 7.2 トレイト定義（確定形）

```rust
pub trait StackStorage<T> {
    fn capacity(&self) -> usize;
    /// Safety: index は不変条件内でのみ渡すこと。
    unsafe fn read_unchecked(&self, idx: usize) -> *const T;
    unsafe fn write_unchecked(&mut self, idx: usize, val: T);
}

#[derive(Clone, Copy, Debug)]
pub enum StackOverflowPolicy {
    Block,
    Grow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PushOutcome {
    Pushed,
    GrewTo { capacity: usize },
}

#[derive(Debug)]
pub enum StackError {
    Empty,
    Closed,
    Disconnected,
    WouldBlock,
    AllocError,
    Full,
}

pub trait StackBackend<T> {
    type Storage: StackStorage<T>;

    fn new(storage: Self::Storage, policy: StackOverflowPolicy) -> Self;

    fn push(&mut self, item: T) -> Result<PushOutcome, StackError>;
    fn pop(&mut self) -> Result<T, StackError>;
    fn peek(&self) -> Option<&T>;

    fn len(&self) -> usize;
    fn capacity(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
    fn is_full(&self) -> bool { self.len() == self.capacity() }
    fn close(&mut self) {}
}
```

- `StackOverflowPolicy` は `Block`（空きが出るまで待機。ただし ISR では `WouldBlock` にフォールバック）と `Grow`（alloc 拡張。失敗時 `AllocError`）のみ提供。`DropNewest` / `DropOldest` は LIFO にそぐわないため扱わない。
- KPI: push/pop の p50/p99、および throughput（Queue と同じ容量・ペイロード）。
- 不変条件: LIFO、`len <= capacity`、`close` 後の push/pop は `Closed` を返す。
- Stack ファサードも `ArcShared` を利用し、`SharedError` → `StackError` のマッピングは Queue と同様（Poison → `Disconnected`、BorrowConflict → `WouldBlock` 等）。

### 7.3 Stack ファサード

```rust
pub struct Stack<T, B: StackBackend<T>> {
    inner: ArcShared<B>,
    _pd: core::marker::PhantomData<T>,
}

impl<T, B: StackBackend<T>> Stack<T, B> {
    pub fn new(shared_backend: ArcShared<B>) -> Self {
        Self { inner: shared_backend, _pd: core::marker::PhantomData }
    }

    pub fn push(&self, item: T) -> Result<PushOutcome, StackError> {
        self.inner
            .try_with_mut(|backend| backend.push(item))
            .map_err(|_| StackError::Disconnected)?
    }

    pub fn pop(&self) -> Result<T, StackError> {
        self.inner
            .try_with_mut(|backend| backend.pop())
            .map_err(|_| StackError::Disconnected)?
    }

    pub fn peek(&self) -> Result<Option<T>, StackError>
    where
        T: Clone,
    {
        self.inner
            .try_with_mut(|backend| Ok(backend.peek().cloned()))
            .map_err(|_| StackError::Disconnected)?
    }
}
```

### 7.4 エラーと ISR ポリシー

**ISR（割り込み文脈）では `OverflowPolicy::Block` はブロックを行わず、必ず `WouldBlock` を返す。**
Queue の場合は `QueueError::WouldBlock`、Stack の場合は `StackError::WouldBlock` を返す。

- `ArcShared` 由来エラーは Queue と対称に扱う：Poisoned → `Disconnected`、BorrowConflict → `WouldBlock`、InterruptContext → `WouldBlock`。
- `StackOverflowPolicy::Block` を採用する実装が ISR から呼ばれた場合も `StackError::WouldBlock` を返し、ブロックしない。
- `close()` 呼び出し後の `push` は `Closed`、残要素が尽き次第 `pop` も `Closed` を返す。

## 8. Close セマンティクス

- Queue: `close()` 呼び出し後の `offer` は `Err(QueueError::Closed)`、残要素が尽き次第 `poll` は `Err(QueueError::Closed)`。
- Stack: `close()` 呼び出し後の `push` は `Err(StackError::Closed)`、残要素が尽き次第 `pop` は `Err(StackError::Closed)`。
- （統一）**ISR では Queue/Stack ともに `OverflowPolicy::Block` は `WouldBlock` を返す**。ブロッキングは発生しない。

## 9. 仕様上の不変条件

- FIFO 系: offer/poll 順序を保持。
- Priority 系: `Ord` に基づく最小値取得が保証される。
- Stack 系: LIFO 順序を保持。
- `len() <= capacity()` は常に成立。
- `OverflowPolicy::Grow` 実装は `offer` 中の再割り当て失敗時に `AllocError` を返す。

## 10. テストおよび受け入れ条件

- **UI テスト**: TypeKey で禁止されている組み合わせ（例: `Queue<T, SpscKey, _>` に複数 producer ヘルパを適用）がコンパイルエラーになることを `ui` テストで確認。
- **SharedError マッピング**: Poison / BorrowConflict / InterruptContext 発生時に意図した `QueueError` / `StackError` が返るユニットテスト。
- **critical-section 再入**: `critical_section::with` 内で `ArcShared::with_mut` をネスト呼びすると `SharedError::BorrowConflict` になることをテスト。
- `embedded-cs` 実装での再入禁止（CS ネスト）: `cs_reentry_returns_borrow_conflict`
- **ベンチマーク**: Queue（MPSC/SPSC/Priority）および Stack の p50/p99 と throughput を測定し、Phase1 ベースラインと ±5% 以内を達成（容量 1k/64k × ペイロード 8B/256B/4KB × producer 数）。
- **thumb check**: `thumbv6m-none-eabi` と `thumbv7em-none-eabihf` で `cargo check --no-default-features --features alloc,<target feature>` が通ること。

## 11. 旧 API からの移行指針（抜粋）

| 旧 API                | 新 API                              | 備考                              |
| --------------------- | ----------------------------------- | --------------------------------- |
| `MpscQueue<T>`        | `Queue<T, MpscKey, _>`              | `type SharedQueue<T, MpscKey, B> = Queue<T, MpscKey, B>` で互換提供 |
| `RingQueue<T>`        | `Queue<T, FifoKey, _>`              | 溢れ政策は `OverflowPolicy` で指定             |
| `PriorityQueue<T>`    | `Queue<T, PriorityKey, _>`          | `PriorityBackend<T>` 実装必須                 |
| `QueueRw` 系トレイト     | 廃止（`QueueBackend` に集約）            | `ArcShared` の `with_mut` を利用               |
| 旧 `Stack` 実装            | `Stack<T, Backend>`（新トレイト準拠）        | `StackBackend` / `StackStorage` を実装する      |

旧 API は `#[deprecated]` を付与し、移行ガイドで `cargo fix` / `sed` 例を提示する。

## 12. モジュール構成（新設計）

- 既存実装は `collections/queue` / `collections/stack` に残し、互換性を保持する。
- 新設計の実装は `collections/queue2` / `collections/stack2` に配置し、移行期間中は並行運用する。
- 旧 API からの巻き取りが完了した段階で統合方針を再評価する。


## 14. ランタイム拡張方針

- コア実装（queue2 / stack2）は `no_std + alloc` のみを前提とし、同期は `ArcShared` 抽象に委譲する。
- Tokio や Embassy 等のランタイムに合わせた Backend / Shared 実装は、`utils-std` / `utils-embedded` など環境別クレートで提供する。
- 追加 Backend は `QueueBackend` / `StackBackend` を実装すればよく、TypeKey / Capability による API 安定性を活かして差し替えが可能。
- 例: `TokioMpscBackend` を `utils-std` に用意し、`Queue<T, MpscKey, TokioMpscBackend<T>>` で利用する。Embassy 向けには `CriticalSectionBackend` を `utils-embedded` に配置する。
**`no_std + alloc` を中核に据え、Backend / Shared の差し替えだけで各ランタイムへ展開できる拡張性を維持する。**

## 13. 開発者向けメモ

- `ArcShared` の切替実装は `shared` モジュール内に閉じ込め、他レイヤには `cfg` を漏らさないこと。
- `PriorityBackend` 実装では `peek_min` の戻りを `Option<&T>` にし、Queue 側で `cloned()` 等を行う。
- `Grow` ポリシーを実装する場合、事前に `try_reserve` 等でメモリ確保を行い、失敗時は `QueueError::AllocError` を返す。
- API ドキュメントには `OfferOutcome` の各バリアントを logging/metrics で扱う例を掲載する。
- Stack でも `ArcShared` の error マッピングは Queue と同じ表を参照し、Poison/BorrowConflict の扱いを統一する。

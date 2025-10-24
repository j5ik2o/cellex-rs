# ADR-003: Queue/Stack リファクタリング Phase1 調査結果

## ステータス

提案中

## コンテキスト

- `docs/design/collections_queue_refactor_plan.md` に定義されたフェーズ1（現状調査と設計整理）の成果物をまとめる。
- 目的は `modules/utils-core` を起点とした Queue/Stack 抽象の現状を可視化し、フェーズ2以降の再設計案（TypeKey、OverflowPolicy、Shared 戦略）を具体化する下地を整えること。
- 調査範囲は `modules/utils-core`（no_std）、`modules/utils-std`、`modules/utils-embedded`、および Queue/Stack を利用するランタイム層（actor-core / actor-std / actor-embedded）。

## 現状分析

### 1. Queue/Stack レイヤ構造

```
Queue/Stack API (RingQueue, MpscQueue, PriorityQueue, Stack)
        │ 依存
        ▼
Backend/Handle (RingHandle, MpscHandle, StackHandle)
        │ 依存
        ▼
Storage 抽象 (QueueStorage, RingBufferStorage, StackStorage)
        │ 依存
        ▼
実データ構造 (RingBuffer, MpscBuffer, StackBuffer)
```

| レイヤ | 代表型/トレイト | 実装例 | 主な課題 |
| --- | --- | --- | --- |
| API | `RingQueue`, `MpscQueue`, `PriorityQueue`, `Stack` | `modules/utils-core/src/collections/queue/ring/ring_queue.rs` 等 | `QueueRw` 実装が自己再帰でスタックオーバーフローの危険あり。能力差異（peek 等）を型で表現できていない。 |
| Backend | `RingBackend`, `MpscBackend`, `StackBackend` | `RingBufferBackend`, `RingStorageBackend` | 戻り値が `QueueError` 固定で政策差分を表現できない。`&self` メソッド主体で同期責務が曖昧。 |
| Storage | `QueueStorage`, `RingBufferStorage`, `StackStorage` | `SpinSyncMutex<RingBuffer<T>>`, `RefCell<RingBuffer<T>>` | `with_read` / `with_write` によるクロージャ委譲で API が複雑化。`QueueStorage` が `ArcShared<SpinSyncMutex<...>>` まで吸収しており階層が不明確。 |
| Buffer | `RingBuffer`, `MpscBuffer`, `StackBuffer` |  | 動的拡張フラグで Overflow 政策を内包しており、政策差分の切り替えが困難。 |

### 2. トレイトおよび抽象の棚卸し

| カテゴリ | 現行トレイト/型 | 観測された課題 | フェーズ2以降の方針 |
| --- | --- | --- | --- |
| Queue の IO | `QueueWriter`, `QueueReader`, `QueueRw` | `QueueRw::offer/poll/clean_up` が自身を再帰呼び出ししておりバグ。能力差を分離できていない。 | `SyncQueueBackend` に集約し、Shared 層で `with_mut` を提供する案に移行。 |
| Handle | `QueueHandle`, `RingHandle`, `MpscHandle`, `StackHandle` | `Shared` トレイトを継承しつつ `Clone` を要求。`QueueHandle` が storage 自体を露出し抽象が崩れる。 | TypeKey + Backend の構成に置き換え、Handle 抽象を段階的に廃止。 |
| Storage | `QueueStorage`, `RingBufferStorage`, `StackStorageBackend` | `SpinSyncMutex` / `RefCell` / `ArcShared` を同列に実装しており同期責務が混在。 | Storage は純粋データ操作に限定し、Shared 層を `ArcShared<T>` に統一。 |
| Error | `QueueError<T>`（`Full/OfferError/Closed/Disconnected`） | Overflow 政策や割り込み失敗を表現できず、ArcShared のエラーをマッピングする余地がない。 | Spec 通り `QueueError` を再設計し、`OverflowPolicy`/`OfferOutcome` と併用する。 |

### 3. Stack 周辺

- `Stack` も Queue と同じ多層構造だが、`StackBackend` が `QueueSize` を再利用しており容量表現が混在。
- `StackError<T>` は `modules/utils-core/src/collections/stack/buffer.rs` に実装されているが、Queue 側と同様に政策や共有抽象を分離できていない。
- フェーズ2で Queue と同じ層構造（Storage → Backend → Shared → API）へ揃える必要がある。

### 4. Shared / 同期戦略

- `Shared` トレイト (`modules/utils-core/src/sync/shared/shared_trait.rs`) を基底に、`ArcShared` (`alloc::sync::Arc` / `alloc::rc::Rc` フォールバック) と `RcShared` が提供されている。
- `SpinSyncMutex`（`spin::Mutex` ラッパー）を `QueueStorage` が直接利用し、`ArcShared<SpinSyncMutex<_>>` を Storage として扱う実装が複数存在。Shared 層と Storage 層が混在している。
- `QueueHandle`/`MpscHandle` は `Shared` を継承し、`ArcShared` 等と同様の責務を持ってしまっている。フェーズ2で `ArcShared<T>` を唯一の Shared エントリポイントにする必要がある。

### 5. no_std 制約と依存

- `modules/utils-core` は `#![no_std]` かつ `alloc` フィーチャのみを想定。同期は `spin` crate に依存し、`critical-section` をビルド時に必須とする。
- `QueueStorage`/`RingBufferStorage` が `RefCell`（`alloc`）と `SpinSyncMutex` の両方に対応し、ターゲット毎に `ArcShared` または `RcShared` を通して共有する設計。環境依存部分が core に流入しており、フェーズ3で `utils-std` / `utils-embedded` に切り出す前提を強める必要がある。
- `modules/utils-std` / `modules/utils-embedded` では Queue 抽象をそのまま再エクスポートしており、現状のトレイト構成を前提に多数の型が存在する。

## TypeKey と能力トレイト案

- TypeKey のベース定義（Spec §2.1）を採用する：

```rust
pub trait TypeKey: 'static {}

pub struct MpscKey;    impl TypeKey for MpscKey {}
pub struct SpscKey;    impl TypeKey for SpscKey {}
pub struct FifoKey;    impl TypeKey for FifoKey {}
pub struct PriorityKey;impl TypeKey for PriorityKey {}
```

- 能力トレイト（`MultiProducer`, `SingleProducer`, `SingleConsumer`, `SupportsPeek`）は下表の契約で整理する。

| TypeKey | Producer 能力 | Consumer 能力 | 追加能力 | 想定用途 |
| --- | --- | --- | --- | --- |
| `MpscKey` | `MultiProducer` | `SingleConsumer` | — | 複数 Producer → 単一 Consumer |
| `SpscKey` | `SingleProducer` | `SingleConsumer` | — | 単一スレッド FIFO |
| `FifoKey` | `SingleProducer` | `SingleConsumer` | — | 固定長 Ring（Embedded 向け） |
| `PriorityKey` | `SingleProducer` | `SingleConsumer` | `SupportsPeek` | 優先度付き Queue |

- Backend 側で `where K: MultiProducer` のように制約を付与し、コンパイル時に誤用を排除する。フェーズ2では Queue API を `Queue<T, K, B>` 形式に再設計し、現在の `RingQueue` / `MpscQueue` は型エイリアス化する。

## OverflowPolicy / OfferOutcome / QueueError の再設計方針

- Spec §4 に従い、Overflow 政策と結果を明示的に表現する。

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverflowPolicy { DropNewest, DropOldest, Block, Grow }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfferOutcome {
  Enqueued,
  DroppedOldest { count: usize },
  DroppedNewest { count: usize },
  GrewTo { capacity: usize },
}
```

- `QueueError` は以下の代表例を含む形へ拡張する（Spec §4.2）：
  - `Full`
  - `Empty`
  - `Closed`
  - `Disconnected`
  - `WouldBlock`
  - `AllocError`

- 現行の `QueueError<T>`（`Full/OfferError/Closed/Disconnected`）との差分を次フェーズで吸収し、Mailbox 実装 (`modules/actor-core/src/api/mailbox/queue_mailbox/base.rs`) が期待する失敗パターン（`Full` / `Closed` / `Disconnected`）を維持しつつ、新たな政策を扱えるようにする。

## SharedError → QueueError マッピング

- Spec §5.1 のマッピングを採用し、Shared 層のエラーを Queue API へ還元する。

| SharedError | QueueError | 想定シナリオ |
| --- | --- | --- |
| `Poisoned` | `Disconnected` | Mutex 毒状態。再利用不可。 |
| `BorrowConflict` | `WouldBlock` | `ArcShared`（std）での `TryLock` 失敗、`critical-section` でのネスト禁止違反。 |
| `InterruptContext` | `WouldBlock` | 割り込み文脈で `with_mut` 不許可。 |

- `ArcShared::with_mut` / `try_with_mut` を Queue API の単一入口にし、既存の `QueueRw::offer/poll` などは段階的に廃止する。

## 影響範囲（主要クレートとファイル）

- `modules/actor-core`
  - `api/mailbox/queue_mailbox/base.rs`, `recv.rs`, `queue_mailbox_producer.rs`: Mailbox API が `QueueRw`/`QueueError` に強く依存。
  - `shared/mailbox/factory.rs`: `QueueHandle` ベースで Mailbox を構築。
- `modules/actor-std`
  - `tokio_mailbox/tokio_queue.rs`, `tokio_priority_mailbox/queues.rs`: `QueueRw` と現在の `ArcSharedRingQueue` を想定。
- `modules/actor-embedded`
  - `local_mailbox/local_queue.rs`, `arc_priority_mailbox/queues.rs`: `RcShared` ベースの Queue 実装に依存。
- `modules/utils-std` / `modules/utils-embedded`
  - `collections/queue/...`: core のトレイト構成をそのまま再エクスポートし、`ArcRingQueue` / `RcRingQueue` など環境別具体型を提供。
- `modules/utils-core`
  - `collections.rs` で大量の再エクスポートを行っており、モジュール再配線（`docs/guides/module_wiring.md`）の規約に抵触。フェーズ2で整理が必要。

## ベースライン計測 (2025-10-23 実施)

- コマンド: `cargo bench -p cellex-utils-core-rs ring_queue_offer_poll`
- 環境: 開発マシン (x86_64, Criterion 0.5, backend plotters)

| ベンチ項目 | 中央値 | 95% 信頼区間 | 備考 |
| --- | --- | --- | --- |
| `ring_queue_offer_poll/rc_refcell` | 697.92 ns | [693.54 ns, 702.74 ns] | `RcShared<RefCell<RingBuffer>>`。前回計測比 +≈2.9%。 |
| `ring_queue_offer_poll/arc_shared_spin` | 723.74 ns | [719.15 ns, 728.23 ns] | `ArcShared<SpinSyncMutex<RingBuffer>>`。`rc_refcell` 比 +3.7%。 |

- KPI として Phase2/3 以降の実装で同条件ベンチを再測し、`±5%` 以内の劣化に抑えることを目標とする。

## 決定事項（Phase1 結果）

1. Queue/Stack の層構造を「Storage → Backend → Shared → API」に整理し、Shared 層を `ArcShared<T>` に集約する方針を採用する。
2. TypeKey + Capability トレイトの組合せで Queue API を型レベル契約に基づかせる。
3. `OverflowPolicy` / `OfferOutcome` / 改訂版 `QueueError` を導入し、政策分岐と Shared エラーを正しく表現する。
4. `QueueWriter` / `QueueReader` / `QueueRw` / `QueueHandle` など現行トレイトはフェーズ2で段階的に廃止・置換する。
5. ベンチマーク `ring_queue_offer_poll` の現行値を KPI ベースラインとして採用する。

## リスクと未解決事項

- `QueueRw` の自己再帰バグは現行実装でもパニックを誘発する恐れがあり、フェーズ2実装に先行して暫定修正が必要になる可能性がある。
- `QueueError` の互換性が Mailbox API に影響するため、`QueueMailbox` でのエラーハンドリング更新とテスト拡充が必須。
- `no_std` 環境での `ArcShared` 切り替え条件（`target_has_atomic = "ptr"`）に伴う条件分岐をどこまで core に残すか要検討。
- `modules/utils-core/src/collections.rs` の再エクスポートが大量で、モジュール配線ガイドラインに違反している。フェーズ2でのリネームと公開範囲調整が必要。

## 次フェーズ (フェーズ2以降) に向けたフォローアップ

1. `SyncQueueBackend` / `QueueStorage` / `QueueError` の再設計を実装し、`ArcShared::with_mut` を通じた同期の統一を行う。
2. TypeKey + Capability を導入した新しい `Queue<T, K, B>` API を試作し、現行の `RingQueue` / `MpscQueue` をエイリアス化。
3. Stack 側の抽象を Queue と揃え、`StackBackend` での `QueueSize` 依存を解消する。
4. Mailbox (`actor-core`) のユニットテストを更新し、新しいエラー/政策を検証するテストケースを追加する。
5. KPI 監視のため、ベンチ結果を `docs/design/collections_queue_spec.md` に追記する仕組みを検討する。

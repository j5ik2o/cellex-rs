# QueueMailbox v2 移行設計メモ（2025-10-24 起案）

## 背景
- 現行の `QueueMailbox<Q, S>` は v1 `QueueRw` トレイトを前提としており、`QueueError<T>` も旧構成に拘束されている。
- v2 への移行では `QueueRwCompat` を経由せず、直接 `v2::collections::queue::SyncQueue` を保持する形へ置き換える必要がある。
- 併せて `OfferOutcome` / `PollOutcome` の情報をレイヤ横断で扱えるようにし、メールボックス周辺のメトリクス／エラー処理を最新仕様に合わせる。

## 目的
1. `QueueMailbox` の内部キューを v2 `SyncQueue` ベースへ差し替え、互換層を取り除く準備を整える。
2. `QueueMailboxProducer` / `QueueMailboxRecv` が `OfferOutcome` / `QueueError` を完全に扱えるようインターフェースを再設計する。
3. メトリクス・デッドレター・スケジューラ通知の流れを統一し、Dropped／Grewイベントを標準で捕捉できるようにする。
4. v1 -> v2 移行を段階的に進めるための実装ステップとテスト計画を明文化する。

## スコープ（Stage 1）
- `QueueMailbox` を `QueueMailboxInner<T, B, S>` のようなジェネリック構成へ分解し、内部で保持するキュー型を抽象化する。
- `QueueMailboxProducer` が `OfferOutcome` を直接受け取り、`DroppedOldest` / `DroppedNewest` / `GrewTo` をメトリクスへ転送する仕組みを組み込む。
- `QueueMailboxRecv` が `QueueError::WouldBlock` を Pending 化し、`Closed` / `Full` 等のエラーと未読メッセージの引き渡しを整理する。
- `QueueError<T>` → `MailboxError` 変換テーブルの草案を docs/design に追記（どのイベントで何を返すか）。

## スコープ（Stage 2 以降の展望）
- `QueueMailbox` が直接 `SyncQueue<T, Backend>` を保持する実装へ切り替える。
- `QueueRwCompat` は Tokio など v2 移行途中のファサード専用として最小限維持し、最終的には廃止できる構成にする。
- `ActorScheduler` / `PriorityMailbox` などの呼び出し元を順次更新し、`QueueRwCompat` 依存箇所を削減する。
- `queue-v1` フィーチャーを deprecate し、最終的に削除。

## 前提となる成果物
- `MetricsEvent::{MailboxDroppedOldest, MailboxDroppedNewest, MailboxGrewTo}` が既に導入済み。
- Tokio 側のメトリクス連携テスト、Scheduler 経由の drop テストが通っている（フェーズ4B）。
- queue-v1 / queue-v2 両ビルドが CI で確認済み。

## 影響範囲
- `QueueMailbox` / `QueueMailboxProducer` / `QueueMailboxRecv` の API 変更。
- `ReadyQueueScheduler` / `PriorityMailbox` / `TestMailboxFactory` など、QueueMailbox を直接利用しているモジュール。
- `QueueError<T>` -> 新しい `MailboxError`（仮称）の整理。

## リスク整理
- 変更規模が大きいので、段階的に進める必要がある。
- `QueueMailbox` が複数箇所で用いられており、互換インターフェースのバランスを崩すとコンパイルエラーが多発する。

## TODO（Stage 1 実装項目）
1. `QueueMailbox` の内部構造を `QueueMailboxCore<Q, S>`（仮称）として切り出し、キュー操作・シグナル・メトリクス・スケジューラ通知を集約する。
2. `QueueMailboxProducer` に `OfferOutcome` を扱う仕組みを導入し、Dropped/Grew 情報をメトリクス／デッドレターへ反映する設計をまとめる。
3. `QueueMailboxRecv` について、`QueueError::WouldBlock` や v2 側の `PollOutcome` を `Poll::Pending`／`MailboxError` へマッピングする方針を文書化し、テスト計画を追加する。
4. `QueueRwCompat` にメトリクスフック／OfferOutcome フックを設定する API を整理し、Tokio/priority からの利用手順を明記する。
5. `QueueError` → `MailboxError` 変換表と、それに対応するユニットテストの網羅方針を設計メモに追記する。

## 2025-10-25 追加メモ: SyncQueue 直接保持構造の骨子

- `MailboxQueueCore` は後続フェーズで "ドライバ" 抽象を介してキューへアクセスする。共通トレイト案:

  ```rust
  pub trait MailboxQueueDriver<M: Element>: Clone {
    fn len(&self) -> usize;
    fn capacity(&self) -> usize;
    fn offer(&self, message: M) -> Result<OfferOutcome, QueueError<M>>;
    fn poll(&self) -> Result<PollOutcome<M>, QueueError<M>>;
    fn close(&self) -> Result<Option<M>, QueueError<M>>;
    fn set_metrics_sink(&self, sink: Option<MetricsSinkShared>);
  }
  ```

  - `PollOutcome<M>` は `SyncQueue` 直結時に `MpscQueue::poll` が返す `QueueError::Empty` / `QueueError::WouldBlock` を `PollOutcome::Empty` / `PollOutcome::Pending` へマップする薄いラッパ。
  - 互換層 (`LegacyQueueDriver`) では `QueueRwCompat` を抱え、`offer` が `Ok(())` を返した場合は `OfferOutcome::Enqueued` を生成する。
  - v2 直結層 (`SyncQueueDriver`) では `SyncQueue<EntryShared<M>, VecRingBackend<_>>` を保持し、`offer` の戻り値や `poll` の `QueueError` をそのまま Mailbox レイヤに伝搬する。

- `MailboxQueueCore<Q, S>` の型パラメータ `Q` は上記ドライバを実装する型に差し替える。これにより `len` / `capacity` / `try_send` / `try_dequeue` / `close` はドライバ経由で `OfferOutcome` / `PollOutcome` を受け取り、メトリクスとスケジューラ通知を処理できる。

- メトリクスフックの扱い:
  - `MailboxQueueCore::set_metrics_sink` はドライバへ委譲し、各ドライバ実装が `QueueRwCompat::set_metrics_sink` または `SyncQueueDriver` 内部の `ArcShared<SpinSyncMutex<Option<MetricsSinkShared>>>` を更新する。
  - `OfferOutcome::{DroppedOldest,DroppedNewest,GrewTo}` はドライバ側でメトリクスイベントへ変換し、`MailboxQueueCore` は `MailboxEnqueued` のみを記録する構造に簡素化できる。

- `QueueMailbox` / `QueueMailboxProducer` / `QueueMailboxRecv` はジェネリクス境界を `Q: MailboxQueueDriver<M>` へ更新し、旧 `QueueRw` 境界を段階的に撤廃する。

## 2025-10-25 追加メモ: MailboxError 雛形

- `MailboxError<M>` のドラフト（後続フェーズで実装想定）:

  ```rust
  pub enum MailboxError<M> {
    QueueFull { policy: OverflowPolicy, preserved: Option<M> },
    Disconnected,
    Closed { last: Option<M> },
    Backpressure,
    ResourceExhausted,
    Internal,
  }
  ```

  - `policy` は `OfferOutcome::DroppedNewest` など発生源のポリシーを保持。
  - `preserved` は `QueueError::Full(message)` のメッセージ再取得用。DropOldest 系では `None`、DropNewest では `Some(message)`。
  - `ResourceExhausted` は `QueueError::AllocError`、`Internal` は `QueueError::OfferError` に対応。

- `QueueError` からの変換テーブル更新案:

  | QueueError / Outcome             | MailboxError                                                  |
  |----------------------------------|---------------------------------------------------------------|
  | `QueueError::Full(message)`      | `MailboxError::QueueFull { policy: OverflowPolicy::DropNewest, preserved: Some(message) }`
  | `OfferOutcome::DroppedOldest`    | `Ok(())`（メトリクスのみ記録）                                |
  | `OfferOutcome::DroppedNewest`    | `MailboxError::QueueFull { policy: OverflowPolicy::DropNewest, preserved: Some(message) }`
  | `OfferOutcome::GrewTo(capacity)` | `Ok(())`（`MailboxGrewTo` を記録）                             |
  | `QueueError::WouldBlock`         | `MailboxError::Backpressure`                                  |
  | `QueueError::Disconnected`       | `MailboxError::Disconnected`                                   |
  | `QueueError::Closed(message)`    | `MailboxError::Closed { last: Some(message) }`                |
  | `QueueError::AllocError(message)`| `MailboxError::ResourceExhausted`（デッドレター対象）           |
  | `QueueError::OfferError(message)`| `MailboxError::Internal`（ログ＋デッドレター）                 |

- `QueueMailboxRecv` は `PollOutcome` から `MailboxError` へ変換するヘルパを備え、`QueueError::Closed` や `Disconnected` を明示的に区別する。

## 2025-10-25 追加メモ: queue-v1 / queue-v2 併存時のドライバ構成

- `queue-v1` ビルド: `MailboxQueueCore` へ渡す `Q` は `LegacyQueueDriver<M>`。
- `queue-v2` ビルド（既定）: 標準は `SyncQueueDriver<EntryShared<M>>`。Tokio/priority 互換ルートは当面 `LegacyQueueDriver` を利用しつつ、逐次 `SyncQueueDriver` へ移行する。
- `LegacyQueueDriver` と `SyncQueueDriver` はどちらも `Clone + Send + Sync` 条件を満たす必要がある。

- テスト指針:
  - ドライバごとの差異を吸収する共通テストセットを `mailbox/queue_mailbox/tests.rs`（新設予定）へ集約し、`#[cfg(feature = "queue-v2")]` で双方のドライバを検証する。
  - 互換層を削除可能になったタイミングで `LegacyQueueDriver` を `deprecated` 化し、段階的な撤廃スケジュールをドキュメントへ追記する。

## TODO（Stage 2 実装項目）
1. `QueueMailbox` を `SyncQueue` 直接保持へ切り替え。
2. `QueueMailboxProducer::try_send` を `OfferOutcome` 駆動に完全移行（DroppedOldest/GrewTo をメトリクス発火、DroppedNewest -> Full エラー返却）。
3. `QueueMailboxRecv` に `PollOutcome`（未定義）レイヤを導入し、closed/dropped の整理を行う。
4. `QueueMailbox` の利用箇所をモジュール毎に v2 へ差し替え（Scheduler -> Priority Mailbox -> Embedded Mailbox の順）。
5. `QueueMailbox` 互換テスト（Signal 違い含む）の追加。

## 要調査メモ
- Deadletter / FailureTelemetry への影響：`QueueError::Full` が `MailboxError::QueueFull` へ移行する場合のログ/テレメトリの再設計。
- Embedded ランタイムでの `SyncQueue` 利用時、`Arc` が使えない場合の互換レイヤ（`ArcShared` の利用範囲）を見極める必要あり。
- `TrySendError` 等、周辺 API のエラーハンドリングを調整する際は tokio-std/crate 利用者への影響も併せて検討。

## 次のステップ
- Stage 1 の TODO を個別の PR に分割し、`queue-v2` を既定とした状態で `QueueMailbox` の内部構造を整える。
- Stage 2 では `QueueRwCompat` 依存削減を優先し、段階的に `SyncQueue` へ移行。完了後にフェーズ5Bのリストと紐付けて残作業を洗い出す。
- 各ステップ完了時に本メモとフェーズ文書を更新し、CI コマンド結果を記録する。

## 現行実装の確認メモ（2025-10-26）

- `QueueMailbox` は `QueueMailboxInternal<Q, S>` をメンバに持ち、内部で v1 `QueueRw<M>` と `QueueError<M>` を直接扱っている。
- `QueueMailboxProducer` / `QueueMailboxRecv` も `QueueRw` 前提で実装されており、`OfferOutcome` / `PollOutcome` の概念はまだ導入されていない。
- メトリクス通知は `QueueMailboxInternal::try_send` 内で `MailboxEnqueued` のみ発火。Dropped/Grew の情報は保持していない。
- `QueueMailbox::close` は `queue.clean_up()` → `signal.notify()` → `closed.set(true)` の順で処理される。v2 へ切り替える際もこの順序を維持する。
- `QueueRwCompat` は Tokio / priority 向けに導入されているが、`QueueMailbox` 自体は `QueueRw` に依存しているため、フェーズ5Bで `SyncQueue` へ切り替える必要がある。

## OfferOutcome / PollOutcome ハンドリング案（暫定）

| v2 戻り値/イベント                     | Mailbox レイヤでの扱い                                      | メトリクス/ログ                          |
|----------------------------------------|--------------------------------------------------------------|------------------------------------------|
| `OfferOutcome::Enqueued`               | `Ok(())`                                                     | `MailboxEnqueued` を記録                 |
| `OfferOutcome::DroppedOldest { count }`| `Ok(())`                                                     | `MailboxDroppedOldest { count }` を記録 |
| `OfferOutcome::DroppedNewest { count }`| `Err(MailboxError::QueueFull { policy: DropNewest })`        | `MailboxDroppedNewest { count }` を記録 |
| `OfferOutcome::GrewTo { capacity }`    | `Ok(())`                                                     | `MailboxGrewTo { capacity }` を記録      |
| `PollOutcome::Item(T)`                 | `Ok(Some(T))`                                                | なし                                     |
| `PollOutcome::WouldWait`/`Idle`        | `Ok(None)` → シグナル待機                                    | なし                                     |
| `PollOutcome::Closed { last: Option<T> }`| `Err(MailboxError::Closed { last })`                         | なし                                     |

## MailboxError 変換テーブル（ドラフト）

| QueueError / Outcome            | MailboxError (案)                              | 備考                                        |
|--------------------------------|-----------------------------------------------|---------------------------------------------|
| `QueueError::Full`             | `MailboxError::QueueFull { policy: DropNewest }` | 既存のバックプレッシャーを QueueFull へ集約 |
| `QueueError::Disconnected`    | `MailboxError::Disconnected`                  | DeadLetter / Scheduler 停止へ通知         |
| `QueueError::Closed`          | `MailboxError::Closed { last: Option<T> }`    | v2 の `PollOutcome::Closed` と整合         |
| `QueueError::WouldBlock`      | `MailboxError::Backpressure`                  | ログ出力のみ。`OfferOutcome::WouldBlock` を想定 |
| `QueueError::AllocError`      | `MailboxError::ResourceExhausted`             | 致命ログ＋デッドレター                     |
| `QueueError::OfferError`      | `MailboxError::Internal`                      | 互換層では発生しない想定（ガード対象）      |

## Stage 1 実装計画（更新）

1. 設計メモ整備（本ドキュメントの更新）
2. メトリクス・エラー処理の抽象化インターフェース検討
3. `QueueMailboxInternal` → `MailboxQueueCore`（仮称）への責務再編
4. OfferOutcome/PollOutcome を受ける `QueueMailboxProducer` / `QueueMailboxRecv` のインターフェース設計
5. MailboxError 変換テーブルとテスト計画の具体化

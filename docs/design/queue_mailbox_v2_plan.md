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
1. `QueueMailbox` の内部構造を `QueueMailboxCore<Q>` のような薄いラッパへ分解する。
2. `QueueMailboxProducer` が `OfferOutcome` を明示的に扱えるようにする（`QueueRw::offer` の戻り値チェックを見直し、Dropped/Grew の情報を取り出す）。
3. `QueueMailboxRecv` が `QueueError::WouldBlock` を `Poll::Pending` に変換するロジックを整理し、整合性テストを追加する。
4. `QueueRwCompat` に `MailboxQueueMetricsHook`（または同等の API）を設け、Drop/Grow イベントを `QueueMailbox` 側からも扱えるようにする。※別PRで調整済み。
5. `QueueError` → `MailboxError` 変換表を docs/design に追記。

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

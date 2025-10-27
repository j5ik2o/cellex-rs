# QueueMailboxCore 分離設計メモ

## ゴール
- System メッセージの予約枠制御・優先処理を `SystemMailboxQueue` に限定し、ユーザーメッセージキューへの直接アクセスを排除する。
- `QueueMailboxCore` は System/User それぞれのキューに対するインターフェースを持ち、どちらのキューにもメトリクス・スケジューラ通知を適用できるようにする。

## アプローチ
1. `MailboxQueueBackend` を System/User 双方で実装し、`QueueMailboxCore` が `SQ`・`UQ` を個別に保持する構造へ変更する。
2. 受信 (`poll`) 時は System キューからの取り出しを優先し、空の場合に User キューへフォールバックする。送信 (`offer`) 時はメッセージ判定に応じて適切なキューへ振り分ける。
3. `SystemMailboxQueue` は System 専用のキューと予約枠・メトリクス更新を担当し、ユーザーメッセージの格納は行わない。ユーザーメッセージは `UserMailboxQueue` が単独で処理する。
4. Embedded/Std/Tokio など各環境の Mailbox 実装は `QueueMailboxCore<SystemMailboxQueue<_>, UserMailboxQueue<_>, _>` を用いる。将来は `LocalMailboxQueue` 等を `SystemMailboxQueue` の差し替えで対応可能とする。

## オープンポイント
- System 判定のロジックをどの層に置くか: 既存同様に `PriorityEnvelope` 判定を `SystemMailboxQueue` 内に保持するか、分離して `QueueMailboxCore` へ移すか。→ 初期案では `SystemMailboxQueue` に保持しつつインターフェースを調整。
- `MailboxQueueBackend` トレイトが想定する API (単一キュー) と矛盾しないか。必要に応じて System/User 用の新しいトレイトか、`QueueMailboxCore` 内での合成パターンを追加検討する。
- Backpressure/OverflowPolicy の扱い: System 用キューが満杯のときの振る舞いを既存テストに合わせて維持する必要がある。

## 検証
- 既存の ReadyQueue / ActorScheduler テストで System 予約枠の挙動が変わらないことを確認する。
- Embedded/Std 双方でビルド・テストが通ること。

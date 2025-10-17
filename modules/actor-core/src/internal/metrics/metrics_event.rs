/// メトリクスイベントを表す種別。
///
/// 現状は概要レベルの区分のみを提供し、詳細なペイロードは後続フェーズで拡張する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricsEvent {
  /// アクターがスケジューラに登録された。
  ActorRegistered,
  /// アクターが停止し、スケジューラから削除された。
  ActorDeregistered,
  /// メールボックスへユーザーメッセージがキューイングされた。
  MailboxEnqueued,
  /// メールボックスからメッセージがデキューされた。
  MailboxDequeued,
  /// テレメトリ呼び出しが実行された。
  TelemetryInvoked,
  /// テレメトリ呼び出しに要した時間(ナノ秒)。
  TelemetryLatencyNanos(u64),
}

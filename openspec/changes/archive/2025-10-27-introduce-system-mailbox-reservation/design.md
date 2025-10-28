# Design: System Message Reservation for QueueMailbox

## Current Behaviour
- `QueueMailboxCore::try_send_mailbox` は単一キュー上で `PriorityEnvelope` の優先度に基づき enqueue するが、容量はユーザーメッセージと共有される。
- System メッセージは高い優先度で並び替えられるものの、キュー自体が満杯 (`QueueError::Full`) の場合は `MailboxError::QueueFull` として拒否される可能性がある。
- Dedicated system channel（protoactor-go の `systemMailbox`）は未実装であり、メールボックス文書でもギャップとして扱われている。

## Goals
1. System メッセージ向けに予約スロット／専用サブキューを追加し、ユーザーメッセージ過多でも制御メッセージが enqueue 可能な状態を保証する。
2. 予約スロットの消費状況をメトリクスで観測できるようにし、飽和時にはログまたはメトリクスで警告を出す。
3. ActorCell/ReadyQueueScheduler 向けのテストで、キューがユーザーメッセージで満杯でも System メッセージが即時に処理されることを確認する。

## Approach Overview
- `QueueMailboxCore` に "system backlog" を表すフィールド（固定サイズリングバッファ、もしくは予約カウンタとセットで保持する VecDeque）を追加し、`PriorityEnvelope::from_system` が呼ばれた場合は専用キューへ格納する。
- `QueueMailboxProducer::try_send_control_with_priority` は既存 API を活用しつつ、内部では system キューへ書き込む。
- dequeue 時 (`QueueMailboxCore::try_dequeue_mailbox`) は system キューを先に確認し、存在する場合はそれを返す。これにより ReadyQueue 経路は変更せずに動作する。
- 予約枠は MailboxOptions で設定可能（デフォルト：System 2 件など）。超過した場合は従来通り `MailboxError::QueueFull` を返す。
- メトリクスについては `MetricsEvent::MailboxEnqueued` に加え、新規イベントまたは既存イベントにフラグを付けて System 予約枠の消費を可視化する（詳細は実装段階で検討）。

## Risks / Open Questions
- 予約枠をどの程度確保するか（デフォルト値・上限値）の調整が必要。
- 複数 System メッセージが短時間に到着した場合のバックプレッシャ。予約枠が枯渇した際の挙動をテストで保証する必要がある。
- メトリクス拡張は `MetricsEvent` の変更を伴うため、既存利用箇所への影響を慎重に把握すること。

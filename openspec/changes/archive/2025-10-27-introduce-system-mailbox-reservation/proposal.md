# Proposal: Dedicated System-Mailbox Capacity

## Why
- 現状の `PriorityEnvelope` による優先度付けは System/User 混在キューで処理されるため、ユーザーメッセージが大量に到着した際に `SystemMessage::Suspend` や `Stop` などクリティカルな制御メッセージが遅延するリスクがある。
- protoactor-go や akka/pekko では system 用キューまたは予約スロットを設けており、制御メッセージが常に即時処理されることが保証されている。
- メールボックス比較ドキュメントでは「専用キュー・予約枠は未導入」と記載されており、設計上のギャップとして認識済み。ランタイム信頼性向上のため早期に解消したい。

## What Changes
- System メッセージ専用の予約スロットまたはサブキューを導入し、ユーザーメッセージが満杯でも System メッセージは常に enqueue 可能にする。
- `QueueMailboxCore` / `QueueMailboxProducer` へ制御メッセージの専用経路を追加し、予約枠が埋まらないようメトリクスを整備する。
- ReadyQueueScheduler／ActorCell テストに制御メッセージが遅延しないことを検証するケースを追加し、旧ドキュメント（`mailbox_expected_features.md` など）を更新する。

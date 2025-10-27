# Actor-Core v2 コレクション移行レポート（最終稿）

- **期間**: 2025-10-24 〜 2025-10-27
- **ステータス**: 完了（2025-10-27 時点）
- **担当**: actor-core チーム（Tokio / Embedded / Utils 各モジュール連携）

## 1. ゴールと最終状態

| 項目 | 最終結果 |
| --- | --- |
| コレクション API | `cellex_utils_core_rs::collections::queue` v2 系に統一済み。旧 `QueueRw` / `ArcMpsc*` はコードベースから撤去。 |
| Mailbox 実装 | `QueueMailbox` / `QueueMailboxProducer` / `QueueMailboxRecv` が `SyncMailboxQueue`（優先度付きは `PriorityMailboxQueue`）を直接保持。互換アダプタ不要。 |
| フィーチャーフラグ | `queue-v1` / `queue-v2` フラグは削除済み。現行ビルドは常に v2 を利用。 |
| Embedded ランタイム | `DefaultMailbox` / `DefaultPriorityMailbox` から `ArcMailboxQueue*` ラッパを廃止し、RawMutex の差し替えは `DefaultSignal<RM>` に集約。 |
| Std ランタイム | Tokio/priority mailboxes は `PriorityMailboxQueue` を直接保持し、`QueueRwCompat` 等の互換層は存在しない。 |
| CI/テスト | `./scripts/ci-check.sh all` が唯一の必須チェック。queue-v1 想定の追加ジョブは不要。 |

## 2. 主な成果

1. **Mailbox コアの単純化**  
   - `QueueMailboxCore` をメッセージ型ジェネリクス毎に `QueueMailboxQueue<M>` トレイトへ委譲する構造に再構成。境界を impl ブロック単位で分離し、`len()` / `capacity()` / `try_send_mailbox()` 呼び出し時のターボフィッシュ指定を解消。

2. **ラッパ型の整理**  
   - Embedded 向け `ArcMailboxQueue` / `ArcPriorityMailboxQueue` を削除し、`SyncMailboxQueue` / `PriorityMailboxQueue` を直接保持。RawMutex パラメータは `ArcSignal<RM>` に限定し、Mailbox/Sender ファクトリのジェネリクス制約を縮小。

3. **優先度付きメールボックスの統一**  
   - `modules/actor-std/src/tokio_priority_mailbox/priority_mailbox_queue.rs` の命名と API を正式採用。Embedded/Tokio の両実装が同一の `PriorityMailboxQueue` を利用し、メトリクスやドロップポリシーの挙動を共有。

4. **エラーモデル更新**  
   - `MailboxError` への変換を `QueueMailboxCore` に集約。`OfferOutcome::{DroppedOldest,GrewTo}` や `QueueError::{Full,Closed,Disconnected,AllocError}` のハンドリングを v2 仕様に合わせて網羅。メトリクス通知 (`MailboxDroppedOldest` / `MailboxGrewTo` 等) を標準化。

5. **テスト整備**  
   - 既存ユニットテストを v2 前提に書き直し、`QueueRw` 依存ケースを削除。`QueueMailbox` 経由の FIFO/ドロップ/優先度挙動を `SyncMailboxQueue` ベースで再検証。Embedded/Tokio いずれも現在の CI タスクで網羅済み。

## 3. アーキテクチャ スナップショット

```
QueueMailbox<Q, S>
  └─ QueueMailboxCore<Q, S>
       ├─ Q: QueueMailboxQueue<M>  // SyncMailboxQueue<M> / PriorityMailboxQueue<M>
       ├─ S: MailboxSignal         // NotifySignal / ArcSignal<RM> / TestSignal
       ├─ metrics_sink: Option<MetricsSinkShared>
       └─ scheduler_hook: Option<ReadyQueueHandle>

PriorityMailboxQueue<M>
  ├─ control_lanes: Vec<SyncMailboxQueue<PriorityEnvelope<M>>>
  └─ regular_lane:  SyncMailboxQueue<PriorityEnvelope<M>>

DefaultMailbox / DefaultPriorityMailbox
  ├─ QueueMailbox<SyncMailboxQueue<_>, DefaultSignal<RM>>
  └─ QueueMailbox<PriorityMailboxQueue<_>, DefaultSignal<RM>>

Tokio Mailbox 系
  └─ QueueMailbox<SyncMailboxQueue<_>, NotifySignal>

TestMailboxFactory
  └─ QueueMailbox<SyncMailboxQueue<_>, TestSignal>
```

## 4. 実施ログ概要

| 日付 | ハイライト |
| --- | --- |
| 2025-10-24 | 旧 API 利用箇所の洗い出し。QueueSize→usize 変換方針決定。`QueueMailboxCore` 再編の設計完了。 |
| 2025-10-25 | `QueueMailbox`/Producer/Recv を v2 仕様に対応。`PriorityMailboxQueue` 実装を統合。queue-v1 フィーチャー撤去方針を決定。 |
| 2025-10-26 | queue-v1 互換コードと `QueueRwCompat` の完全撤去。Tokio/Embedded ファクトリを v2 へ切替。CI 構成を v2 のみへ更新。 |
| 2025-10-27 | Embedded `Arc*` ラッパ削除、ドキュメント更新。最終 CI (`./scripts/ci-check.sh all`) にて完了検証。 |

## 5. フォローアップ / 追加タスク

- 現時点で必須の残タスクはありません。性能観測や更なるベンチマークは任意で実施してください。
- 過去ログ・互換レイヤに関する記録は `docs/sources/nexus-actor-rs/` にアーカイブ済みです。

## 6. 参照

- `modules/actor-core/src/api/mailbox/queue_mailbox/{core.rs,base.rs,queue_mailbox_producer.rs,recv.rs}`
- `modules/actor-std/src/tokio_priority_mailbox/priority_mailbox_queue.rs`
- `modules/actor-embedded/src/default_mailbox/{default_mailbox_impl.rs,factory.rs,sender.rs}`
- `modules/actor-embedded/src/default_priority_mailbox/{factory.rs,mailbox.rs,sender.rs}`
- CI: `./scripts/ci-check.sh all`

---

> この文書は Actor-Core v2 への移行作業が完了したことを記録する最終レポートです。追加の修正が必要になった場合は、別途タスク文書を作成して追跡してください。


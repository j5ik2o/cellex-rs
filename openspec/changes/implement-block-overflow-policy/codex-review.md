# Review: implement-block-overflow-policy

## 指摘事項

1. **致命的** – Blockポリシーの真の待機がMailbox経路で実現できない  
   提案では `SyncAdapterQueueBackend::offer` に待機処理を追加するとしていますが、実際に `OverflowPolicy::Block` を利用している `UserMailboxQueue` は同期キュー (`MpscQueue`) と `VecRingBackend` を直接呼び出しており、`SyncAdapterQueueBackend` を経由していません。したがって提案どおりに実装しても、`QueueMailbox` からの `offer` はこれまで通り即座に `QueueError::Full` を返し続けます。  
   - `UserMailboxQueue::offer` は同期キューに対して `self.queue.offer(...)` を呼び出すだけで、非同期待機の余地がありません。citemodules/actor-core/src/api/mailbox/queue_mailbox/user_mailbox_queue.rs:132  
   - `SyncQueue::offer` は単に `SyncQueueBackend::offer` を呼ぶだけで待機処理を持ちません。citemodules/utils-core/src/collections/queue/sync_queue.rs:44  
   - `VecRingBackend` の Block 分岐は従来どおり `QueueError::Full` を返すだけです。citemodules/utils-core/src/collections/queue/backend/vec_ring_backend.rs:44  
   Block を本当に待機させたいのであれば、同期APIをどのように非同期化するのか（`SyncQueue` を Async 版に置き換えるのか、もしくは `VecRingBackend` 側で待機ロジックを実装するのか）を明確にする必要があります。

2. **重大** – 既存の async 経路で Block がすでに待機している事実と整合していない  
   現状でも `AsyncQueue::offer_shared` は `guard.prepare_producer_wait()` を呼び出し、Block ポリシーなら `WaitQueue` を通じて非同期待機しています。citemodules/utils-core/src/collections/queue/async_queue.rs:35-52  
   `SyncAdapterQueueBackend::prepare_producer_wait` も Block 時にハンドルを返すよう既に実装されています。citemodules/utils-core/src/collections/queue/backend/sync_adapter_queue_backend.rs:116-121  
   提案は「`prepare_producer_wait` が呼ばれていない」という前提で書かれていますが、実際には利用されており、ここを無視して `offer` 側にも待機処理を追加すると責務の重複や二重待機の混乱を招きかねません。現行動作を正しく把握した上で、どこを改善すべきか整理し直してください。

3. **重要** – `WaitHandle::wait()` というAPIは存在しない  
   提案・設計・タスクで共通して「`handle.wait().await`」と記載されていますが、`WaitHandle` は Future 実装を提供するだけで `wait()` メソッドは定義されていません。citemodules/utils-core/src/collections/wait/handle.rs:7  
   このままでは実装タスクが成立せず、仕様にも誤情報が残ります。`waiter.await` のように正しい呼び出し方へ修正してください。

## フォローアップ質問

- Block ポリシーの待機を同期キュー経由でも提供したい場合、Mailbox 側の API を非同期化する方針なのか、それとも `VecRingBackend` 自体を async 対応させるのか、狙いのアーキテクチャを確認したいです。


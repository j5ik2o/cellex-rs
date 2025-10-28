# Spec Delta: mailbox-overflow-handling

**Change:** implement-block-overflow-policy  
**Capability:** mailbox-overflow-handling  
**Status:** Draft

## Overview

Mailbox が `OverflowPolicy::Block` を使用する際に、producer がキュー容量を超えた場合は非同期待機し、空きができたら再開する挙動へ移行する。あわせて Mailbox API を async 化し、`AsyncQueue` + `SyncAdapterQueueBackend` による既存の待機機構を活用する。

---

## ADDED Requirements

### Requirement: Async Mailbox Queue API

Mailbox queue backends SHALL expose asynchronous enqueue/dequeue operations so that overflow handling can suspend tasks without blocking the underlying thread.

#### Scenario: MailboxQueueBackend offers asynchronously

**Given**:  
- A type implementing `MailboxQueueBackend<M>`  
- The trait methods `offer`, `poll`, `close` are declared as `async fn`

**When**:  
- `offer` is invoked while the underlying queue is full and configured with `OverflowPolicy::Block`

**Then**:  
- The call returns a `Future` that remains `Pending` until space becomes available  
- The caller MUST `await` the future to complete the enqueue  
- No OS thread is blocked while waiting

**Acceptance Criteria**:  
- `modules/actor-core/src/api/mailbox/queue_mailbox/backend.rs` defines `MailboxQueueBackend` using `#[async_trait(?Send)]` with `async fn offer/poll/close`.  
- All concrete implementations (`UserMailboxQueue`, system mailbox queues, test stubs) compile with the async signatures.  
- Callers of `MailboxQueueBackend::offer` are updated to `await` the returned future.

---

## MODIFIED Requirements

### Requirement: Block Overflow Policy Behavior

The system SHALL implement true asynchronous blocking behavior for `OverflowPolicy::Block` in the Mailbox pipeline using the shared `AsyncQueue` infrastructure.

#### Scenario: Producer blocks when queue is full

**Given**:  
- `UserMailboxQueue` backed by `AsyncQueue<EntryShared<M>, SyncAdapterQueueBackend<VecRingBackend<EntryShared<M>>>>`  
- Queue capacity is 1 and contains 1 message  
- The overflow policy is `OverflowPolicy::Block`

**When**:  
- A producer invokes `MailboxQueueBackend::offer` with a second message

**Then**:  
- The returned future stays `Pending` (no immediate `Err(QueueError::Full)`)  
- The underlying `AsyncQueue::offer_shared` registers a producer waiter via `prepare_producer_wait()`  
- The producer task yields without blocking the thread

**Acceptance Criteria**:  
- `UserMailboxQueue::offer` awaits `self.queue.offer(...)` (where `self.queue` is an `AsyncQueue`).  
- `SyncAdapterQueueBackend::prepare_producer_wait()` continues to return a waiter when overflow policy is `Block` and the backend is not closed.  
- There exists an async unit test covering this scenario (`offer_blocks_until_space_available_async` or equivalent).

---

#### Scenario: Producer resumes when space becomes available

**Given**:  
- A producer is awaiting completion of `offer` under Block policy  
- A consumer is able to remove an item from the same queue

**When**:  
- The consumer awaits `MailboxQueueBackend::poll` and removes the existing item

**Then**:  
- `SyncAdapterQueueBackend::poll` invokes `notify_producer_waiter()`  
- The waiting producer future is woken and retries the enqueue internally  
- The producer `await` resolves to `Ok(OfferOutcome::Enqueued)`

**Acceptance Criteria**:  
- `QueueMailboxCore::try_dequeue_mailbox` awaits the user/system queues and triggers `notify_producer_waiter()` via the adapter.  
- Integration test via `QueueMailbox` verifies that the third offer completes only after a poll frees capacity.

---

#### Scenario: Multiple producers wait in FIFO order

**Given**:  
- Capacity-1 queue, Block policy  
- Three producers call `offer` sequentially while the queue remains full  
- Each producer awaits the returned future

**When**:  
- The consumer polls three times (awaiting between polls)

**Then**:  
- Producers complete in the same order they awaited (FIFO)  
- No producer starves or is awakened out of order

**Acceptance Criteria**:  
- `WaitQueue` continues to guarantee FIFO semantics for producer waiters.  
- There is a test spawning multiple async tasks that verifies completion order through assertions.  
- Documentation (`mailbox_expected_features.md`) notes FIFO fairness for Block policy.

---

#### Scenario: Waiting producers receive error on queue close

**Given**:  
- A producer is awaiting `offer` under Block policy  
- The queue is closed while still full

**When**:  
- `MailboxQueueBackend::close` is awaited

**Then**:  
- `SyncAdapterQueueBackend::close` calls `fail_all_waiters(|| QueueError::Disconnected)`  
- The awaiting producer future resolves to `Err(QueueError::Disconnected)`  
- No producer remains indefinitely blocked

**Acceptance Criteria**:  
- `UserMailboxQueue::close` awaits the underlying queue and propagates the error.  
- Async unit test validates that closing the queue wakes waiting producers with `QueueError::Disconnected`.

---

### Requirement: Non-Block Policies Unchanged

Policies other than `Block` MUST retain their existing semantics when invoked through the async Mailbox pipeline.

#### Scenario: DropOldest continues to drop oldest item

**Given**:  
- Capacity 2 queue with `OverflowPolicy::DropOldest`  
- Queue contains [1, 2]

**When**:  
- `offer(3)` is awaited

**Then**:  
- Item 1 is dropped  
- Item 3 is enqueued  
- Result is `Ok(OfferOutcome::DroppedOldest { count: 1 })`

**Acceptance Criteria**:  
- Async tests mirror the previous synchronous assertions for DropOldest/DropNewest/Grow.  
- No additional awaits are introduced for non-full queues beyond the necessary async call overhead.

---

## Implementation Notes

- `MailboxQueueBackend` async 化により、Mailbox 呼び出し側もすべて async API (`await`) へ移行する。必要に応じて Transitional wrapper (`block_on`) を短期提供してもよいが、最終的には async パスが正規になる。
- `UserMailboxQueue` は `ArcShared<SpinAsyncMutex<SyncAdapterQueueBackend<VecRingBackend>>>` を所有し、`AsyncQueue::new_mpsc` で共有。Block ポリシーの待機は `AsyncQueue` 内部実装に依拠する。
- System mailbox など即時応答が望ましいキューは `async fn` 内で即時 `Ok` を返して問題無い。

## Testing Strategy

1. **Async unit tests**: `UserMailboxQueue` レベルで Block 待機／FIFO／close エラーを検証。
2. **Integration tests**: `QueueMailbox`（system queue 有無両方）で producer が待機することを確認。
3. **Regression**: Drop/Grow ポリシーの動作が変わらないことを async テストで再確認。
4. **CI**: `cargo test --workspace`, `cargo clippy --workspace --all-targets`, `./scripts/ci-check.sh all`, クロスコンパイル 2 ターゲット。

## Migration Guide

- 旧 API（同期版 `offer`/`poll`）は削除されるため、呼び出し側は `await` へ書き換える。  
- 同期文脈から呼び出す必要がある箇所は、`tokio::spawn_blocking` や `block_on(async { ... })` でラップして暫定対応し、最終的に全経路を async 対応へ移行する。
- テストは `#[tokio::test]` または executor を明示して async に書き換える。

## Validation Criteria

- Mailbox 経路の Block ポリシーが待機 → poll 後に再開することを確認するテストが追加されている。  
- `docs/design/mailbox_expected_features.md` の Block 項目が ✅ へ更新。  
- `openspec validate implement-block-overflow-policy --strict` が成功。  
- CI / クロスコンパイルが引き続き成功。

## References

- `modules/utils-core/src/collections/queue/async_queue.rs`  
- `modules/utils-core/src/collections/queue/backend/sync_adapter_queue_backend.rs`  
- `modules/utils-core/src/wait_queue/`  
- `modules/actor-core/src/api/mailbox/queue_mailbox/*.rs`  
- `docs/design/mailbox_expected_features.md`  
- `docs/sources/protoactor-go/actor/bounded.go`  
- `docs/sources/pekko/docs/src/main/paradox/mailboxes.md`

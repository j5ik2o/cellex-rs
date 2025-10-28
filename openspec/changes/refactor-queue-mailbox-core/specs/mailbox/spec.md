# Capability: Queue Mailbox Core Separation

Articulate how the mailbox core composes system and user queues with clear responsibilities.

## ADDED Requirements

### Requirement: Queue mailbox core composes independent system and user queues
- The mailbox core SHALL accept distinct system and user queue implementations through its generics (e.g., `QueueMailboxCore<SQ, UQ, S>`).
- The core SHALL route enqueue/dequeue operations to the system queue first and fall back to the user queue when appropriate, without either queue needing to know about the other.

#### Scenario: Core polls system queue before user queue
```
Given a mailbox core instantiated with separate system and user queues
When a dequeue operation is performed
Then the system queue is polled first, and only if it is empty is the user queue polled
```

### Requirement: System mailbox queue owns only system reservations
- `SystemMailboxQueue` (or equivalent type) SHALL be responsible solely for system message reservation and metrics, and SHALL NOT wrap or store a user queue internally.
- The system queue SHALL expose APIs that operate exclusively on system messages, delegating user message handling to the provided user queue.

#### Scenario: System queue accepts system messages without accessing user queue internals
```
Given a system queue instance
When a system message is enqueued
Then the operation succeeds without requiring access to any user queue state
```

### Requirement: Preserve existing user queue behaviour while enabling system queue swap
- Existing `UserMailboxQueue` semantics (capacity, overflow policy, metrics propagation) SHALL remain unchanged by the refactor.
- The new structure SHALL allow substituting alternative system queue implementations (e.g., embedded-specific) without modifying user queue code.

#### Scenario: System queue swap does not modify user queue contract
```
Given a custom system queue implementation plugged into the mailbox core
When the user queue handles user messages
Then its API and behaviour match the pre-refactor expectations for capacity and overflow handling
```

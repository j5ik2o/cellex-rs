# Capability: System Mailbox Reservation

Introduce reserved capacity so that System messages are never starved by user traffic.

## ADDED Requirements

### Requirement: Ensure system messages bypass user capacity limits
- The mailbox SHALL provide reserved capacity for system messages such that enqueuing a `SystemMessage` never fails while user slots are full, unless the reserved capacity is also exhausted.
- The reserved capacity SHALL be configurable via `MailboxOptions`, with a sensible default (e.g., at least 1 slot).

#### Scenario: System message enqueues when user queue is full
```
Given a mailbox whose user capacity is filled with regular messages
And the system reservation has available slots
When a SystemMessage::Stop is enqueued
Then the enqueue succeeds and the message is accepted despite user backlog
```

#### Scenario: Reservation exhaustion reports queue full
```
Given a mailbox whose user capacity is full
And the system reservation is also exhausted
When an additional SystemMessage is enqueued
Then the mailbox returns a queue full error specific to system reservation exhaustion
```

### Requirement: Dequeue system messages before user messages
- The mailbox SHALL dequeue and deliver pending system messages before user messages, ensuring immediate processing once a worker becomes available.

#### Scenario: System message is processed first
```
Given a mailbox containing both system and user messages
When the scheduler dequeues messages for processing
Then the system message is delivered before any user message
```

### Requirement: Expose reservation activity via metrics
- The runtime SHALL emit metrics whenever a system reservation slot is consumed or when reservation exhaustion occurs.

#### Scenario: Metrics track reservation usage
```
Given a metrics sink attached to the mailbox
When system messages consume reserved capacity
Then metrics events record the reservation usage and exhaustion counts
```

### Requirement: Provide regression tests to cover reservation behaviour
- Tests SHALL cover user-queue saturation, reservation exhaustion, and scheduler ordering to prevent regressions.

#### Scenario: Integration test ensures system message delivery under saturation
```
Given a ReadyQueueScheduler with actors whose user queues are saturated
When a SystemMessage::Suspend is sent
Then the test asserts the system message is processed without delay and logs contain the expected metrics
```

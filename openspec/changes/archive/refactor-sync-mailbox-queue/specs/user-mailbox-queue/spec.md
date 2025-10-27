# Capability: User Mailbox Queue Naming

Clarify that the primary queue driver is dedicated to user messages and named accordingly.

## ADDED Requirements

### Requirement: Provide a user-specific mailbox queue abstraction
- The runtime SHALL expose a queue driver named `UserMailboxQueue` (or equivalent public alias) to store user messages independent of system lanes.
- The user queue SHALL encapsulate only user message responsibilities (enqueue/dequeue, capacity, overflow policy) and MUST NOT embed system-reservation behaviour.

#### Scenario: User queue module exports user-centric types
```
Given a consumer importing the mailbox queue module
When they access the user-facing queue driver and aliases
Then the exported names follow the `UserMailbox*` prefix and carry no `SyncMailbox*` identifiers
```

### Requirement: Document the separation between user and system queues
- Developer documentation and inline comments SHALL describe `UserMailboxQueue` as the foundation for user mailboxes, with system behaviour layered via `SystemMailboxQueue` or similar wrappers.

#### Scenario: Docs explain user/system split
```
Given the mailbox module documentation
When a maintainer reads the description of the queue driver
Then it states that user and system queues are distinct, and `UserMailboxQueue` handles only user messages
```

### Requirement: Maintain behavioural parity during rename
- The rename SHALL preserve existing runtime behaviour and tests; only naming/clarity updates are permitted.

#### Scenario: Regression tests remain unchanged
```
Given the existing mailbox behaviour tests
When the rename is applied and the suite is executed
Then all relevant tests pass without behavioural modifications to enqueue/dequeue semantics
```

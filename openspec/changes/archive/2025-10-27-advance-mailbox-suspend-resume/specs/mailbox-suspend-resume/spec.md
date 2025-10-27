# Capability: Mailbox Suspend/Resume Completion

The runtime must expose a fully integrated suspend/resume flow that aligns with the design plan and current implementation progress (~75%).

## ADDED Requirements

### Requirement: Record suspension duration when a clock is available
- The runtime SHALL include suspension duration (in nanoseconds) in `MetricsEvent::MailboxSuspended` and `MetricsEvent::MailboxResumed` when `SuspensionClockShared` yields timestamps.
- When a clock is unavailable (`SuspensionClockShared::null()`), the runtime SHALL still emit the events with accumulated counts but omit duration fields.

#### Scenario: Duration is recorded with a mock clock
```
Given an actor running under ReadyQueueScheduler with a mock SuspensionClock
When the actor receives SystemMessage::Suspend and later SystemMessage::Resume after N nanoseconds
Then a metrics sink receives MailboxSuspended / MailboxResumed events containing the measured duration N
```

#### Scenario: Duration omitted without a clock
```
Given an actor running with SuspensionClockShared::null()
When the actor is suspended and resumed
Then emitted metrics events contain suspend/resume counts but no duration field
```

### Requirement: Remove suspended mailboxes from the ready queue until resume conditions are met
- The ReadyQueueCoordinator SHALL remove a mailbox from the ready queue when it receives `InvokeResult::Suspended`.
- The coordinator SHALL re-register the mailbox only when the associated `ResumeCondition` (After/ExternalSignal/WhenCapacityAvailable) is satisfied.

#### Scenario: Resume after deadline
```
Given a mailbox suspended with ResumeCondition::After(Δ)
When the suspension clock advances beyond Δ
Then ReadyQueueCoordinator re-registers the mailbox exactly once and the actor processes pending user messages
```

#### Scenario: Resume on external signal
```
Given a mailbox suspended with ResumeCondition::ExternalSignal(key)
When ReadyQueueCoordinator receives the corresponding resume signal
Then the mailbox is re-queued and pending user messages are delivered
```

#### Scenario: Capacity-based resume
```
Given a mailbox suspended with ResumeCondition::WhenCapacityAvailable
When the actor has capacity to process messages
Then the mailbox is re-queued and processes at least one pending message on the next drain
```

### Requirement: Provide regression tests for multi-actor suspend/resume flows
- The project SHALL include integration tests covering concurrent suspend/resume operations across multiple actors, including backpressure scenarios.
- Tests SHALL assert that metrics, ready queue state, and message delivery all align with the scenarios defined above.

#### Scenario: Dual actors suspended and resumed independently
```
Given two actors suspended with different resume conditions
When each condition is satisfied independently
Then each actor resumes without affecting the other and metrics reflect two separate suspend/resume cycles
```

#### Scenario: Backpressure release delivers queued messages
```
Given an actor suspended due to backpressure and holding pending user messages
When backpressure is relieved and the resume condition is satisfied
Then all pending user messages are delivered and metrics record the resume event
```

### Requirement: Document the completed suspend/resume design
- The design documents SHALL describe the final suspend/resume flow, including metrics behavior and ready queue integration.

#### Scenario: Design plan updated post-completion
```
Given docs/design/mailbox_suspend_resume_plan.md
When the implementation reaches 100% completion
Then the document reflects the finished architecture, remaining risks, and references to the new tests and metrics
```

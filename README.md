# cellex-rs

[![ci](https://github.com/j5ik2o/cellex-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/j5ik2o/cellex-rs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/cellex-actor-core-rs.svg)](https://crates.io/crates/cellex-actor-core-rs)
[![docs.rs](https://docs.rs/cellex-actor-core-rs/badge.svg)](https://docs.rs/cellex-actor-core-rs)
[![Renovate](https://img.shields.io/badge/renovate-enabled-brightgreen.svg)](https://renovatebot.com)
[![dependency status](https://deps.rs/repo/github/j5ik2o/cellex-rs/status.svg)](https://deps.rs/repo/github/j5ik2o/cellex-rs)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![License](https://img.shields.io/badge/License-APACHE2.0-blue.svg)](https://opensource.org/licenses/apache-2-0)
[![](https://tokei.rs/b1/github/j5ik2o/cellex-rs)](https://github.com/XAMPPRocky/tokei)

> Typed, async-first actor runtime families for Rust — designed to scale from embedded MCUs to distributed clusters.

For the Japanese edition of this document, see `README.ja.md`.

## Table of Contents
- [At a Glance](#at-a-glance)
- [Quick Start](#quick-start)
- [Core Capabilities](#core-capabilities)
- [Architecture Overview](#architecture-overview)
- [Development Workflow](#development-workflow)
- [Name & Concept](#name--concept)
- [Project Status](#project-status)
- [Further Reading](#further-reading)
- [License](#license)

## At a Glance

| Theme | Details |
| --- | --- |
| Typed behaviours | `Behavior<U, R>` DSL with Akka/Pekko-like semantics, `Context<'_, '_, U, R>` for scoped access, and `ActorRef<U, R>` for type-safe messaging |
| Runtime portability | Works on `std`, `no_std + alloc`, Tokio, Embassy, and RP2040/RP2350-class MCUs |
| Supervision & resiliency | Guardian hierarchies, restart/resume/stop directives, escalation sinks, and watch/unwatch notifications |
| Scheduling | Priority-aware mailboxes, async `dispatch_next` / `run_until` APIs, and blocking loops for bare-metal hosts |
| Ecosystem | Core runtime (`actor-core`), Tokio adapters (`actor-std`), embedded adapters (`actor-embedded`), cluster/remote modules, and shared utilities |

## Quick Start

### Requirements
- Rust stable toolchain (see `rust-toolchain.toml`)
- `cargo` and `rustup`
- Optional: `tokio` for host runtimes, `embassy-executor` for MCU targets

### Install the crates

```shell
cargo add cellex-actor-core-rs
# For Tokio-based hosts
cargo add cellex-actor-std-rs --features rt-multi-thread
```

### Minimal typed example (Tokio)

```rust
use cellex_actor_core_rs::{ActorSystem, Behaviors, MailboxOptions, Props};
use cellex_actor_std_rs::TokioMailboxRuntime;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let mut system: ActorSystem<u32, _> = ActorSystem::new(TokioMailboxRuntime);
  let mut root = system.root_context();

  let props = Props::with_behavior(MailboxOptions::default(), || {
    Behaviors::receive(|_ctx, value: u32| {
      println!("received: {value}");
      Ok(Behaviors::same())
    })
  });

  let actor = root.spawn(props)?;
  actor.tell(42)?;
  root.dispatch_next().await?; // process one envelope

  Ok(())
}
```

### Cross-checks for embedded builds

```shell
# RP2040 (thumbv6m)
cargo check -p cellex-actor-core-rs --target thumbv6m-none-eabi
# RP2350-class (thumbv8m.main)
cargo check -p cellex-actor-core-rs --target thumbv8m.main-none-eabi
```

## Core Capabilities

- **Typed Actor DSL** — `Behavior`, `BehaviorDirective`, and `Props` help model actor lifecycles with pure functions.
- **Priority Mailboxes** — system envelopes and user envelopes share a mailbox while honouring control-message priority.
- **Supervision Hierarchy** — guardians manage child actors, watchers, and escalation pathways with pluggable strategies.
- **Async Scheduling** — `run_until`, `run_forever`, and blocking loops cover async runtimes, cooperative loops, and MCU main threads.
- **Shared Abstractions** — `MapSystemShared`, `ReceiveTimeoutFactoryShared`, and `ArcShared` enable lock-aware sharing across std/embedded builds.
- **Extensibility** — extensions registry, failure event hub, and remote/cluster modules prepare cellex for distributed deployments.

## Architecture Overview

| Path | What lives here |
| --- | --- |
| `modules/actor-core` | Core typed runtime, behaviours, scheduler, guardians, and mailbox infrastructure |
| `modules/actor-std` | Tokio-based mailbox factories, runtime drivers, and host integrations |
| `modules/actor-embedded` | `no_std + alloc` adapters, Embassy dispatcher helpers, MCU examples |
| `modules/remote-core` / `remote-std` | gRPC transport, remote endpoint supervision |
| `modules/cluster-core` | Gossip-based membership and sharding primitives |
| `modules/utils-*` | Shared utilities (`ArcShared`, queues, alloc-aware helpers) |
| `docs/design` | Current design notes and transition plans (dispatch, typed DSL, mailbox split, etc.) |
| `docs/worknotes` | Engineering notes and how-to guides (Tokio/Embassy drivers, roadmap fragments) |

## Development Workflow

| Purpose | Command |
| --- | --- |
| Format | `cargo +nightly fmt` or `makers fmt` |
| Lint | `cargo clippy --workspace --all-targets` |
| Test (host) | `cargo test --workspace` |
| Coverage | `cargo make coverage` or `./coverage.sh` |
| Cross-check (RP2040/RP2350) | see [Quick Start](#quick-start) |

## Name & Concept

- **Etymology:** `cellex = cell + ex`. `cell` reflects autonomous actors; `ex` (Latin: *outward*, *beyond*, *exchange*) highlights communication across clear boundaries.
- **Meaning layers:**
  1. *Cell Exchange* — message passing mirrors substance exchange across cell membranes.
  2. *Cell Execute* — actors run in parallel, self-directed like cells in a living organism.
  3. *Cell Exceed* — cooperation among actors produces emergent behaviour beyond individual components.
- **Aesthetics:** pronounce it `cel-lex`. The familiar “cel” plus the energetic “lex” (echoing Latin *rex*) underline approachability and resilience.
- **Project mantra:** “Like cells in a living organism, each actor in cellex operates independently yet communicates seamlessly, creating emergent intelligence through distributed coordination.”

## Project Status

- `QueueMailbox::recv` returns `Result<M, QueueError<M>>`; handle non-`Ok` as mailbox closure/disconnect signals.
- `PriorityScheduler::dispatch_all` is deprecated. Prefer `dispatch_next`, `run_until`, or `run_forever` per [dispatch transition guide](docs/design/2025-10-07-dispatch-transition.md).
- Typed DSL is available; upcoming work focuses on richer `map_system` adapters and typed system-event enums (see [Typed DSL MUST guide](docs/worknotes/2025-10-08-typed-dsl-claude-must.md)).

## Further Reading

- [Typed Actor 設計メモ](docs/design/2025-10-07-typed-actor-plan.md): design direction for behaviours, contexts, and system-event mapping.
- [Dispatcher Runtime ポリシー](docs/sources/nexus-actor-rs/docs/dispatcher_runtime_policy.md): legacy (nexus-era) shutdown guidance; concepts still apply but APIs differ.
- [ベンチマークダッシュボード](https://j5ik2o.github.io/cellex-rs/bench_dashboard.html): weekly performance snapshots (`benchmarks/history/bench_history.csv`).
- [ActorContext ロック計測レポート](docs/sources/nexus-actor-rs/docs/benchmarks/tracing_actor_context.md): lock-wait analysis; translate API names when using cellex.
- [ReceiveTimeout DelayQueue PoC](docs/sources/nexus-actor-rs/docs/benchmarks/receive_timeout_delayqueue.md): delay queue experiments for receive timeouts.
- [Actor トレイト統一リリースノート](docs/sources/nexus-actor-rs/docs/releases/2025-09-26-actor-trait-unification.md): background on removing `BaseActor` and adding `ActorSpawnerExt`.
- [レガシーサンプル一覧](docs/sources/nexus-actor-rs/docs/legacy_examples.md): legacy samples from the `nexus` era for migration reference.
- [Tokio dispatcher how-to](docs/worknotes/2025-10-07-tokio-dispatcher.md) / [Embassy dispatcher how-to](docs/worknotes/2025-10-07-embassy-dispatcher.md).
- `modules/actor-embedded/examples/embassy_run_forever.rs`: minimal Embassy integration sample.

## License

Dual-licensed under MIT and Apache-2.0. You may choose either license to govern your use of cellex.

# Gemini Code-Aware Context: cellex-rs

## Project Overview

`cellex-rs` is a typed, asynchronous-first actor runtime framework for Rust. It is designed for high performance and scalability, enabling developers to build applications that run on diverse platforms, from embedded microcontrollers (like RP2040) to distributed clusters.

The project is inspired by actor systems like Akka and Pekko, providing a `Behavior`-based DSL for defining actor logic, type-safe messaging via `ActorRef`, and a robust supervision hierarchy for building resilient systems.

**Key Architectural Features:**

*   **Workspace Structure:** The project is a Rust workspace composed of multiple crates located in the `modules/` directory.
    *   `actor-core`: The core actor runtime, scheduler, and mailbox infrastructure.
    *   `actor-std`: Adapters for the Tokio runtime.
    *   `actor-embedded`: Adapters for `no_std + alloc` environments, including Embassy.
    *   `serialization-*`: Crates for message serialization (e.g., JSON, Postcard).
    *   `utils-*`: Shared utilities for both `std` and `no_std` environments.
*   **Platform Portability:** A core design principle is the ability to run on standard OSs (via `tokio`) and bare-metal systems (via `embassy` or direct loops). This is achieved through abstractions like `Shared<T>` (`ArcShared`, `RcShared`) that provide a consistent API across different memory management models.
*   **Strict Conventions:** The project enforces a rigorous set of coding standards through custom lints (`lints/` directory) and policy-checking scripts (`scripts/`).

## Building and Running

The project uses a centralized script, `scripts/ci-check.sh`, as the primary entry point for all build, lint, and test operations. This script is used in the CI pipeline and is the most reliable way to validate changes.

**Primary Commands:**

*   **Run all CI checks:**
    ```bash
    ./scripts/ci-check.sh all
    ```

*   **Format code:**
    ```bash
    cargo +nightly fmt
    ```

*   **Linting:**
    ```bash
    ./scripts/ci-check.sh lint
    ```
    This runs `cargo clippy` and custom `dylint` checks across the workspace.

*   **Testing (Host):**
    ```bash
    ./scripts/ci-check.sh test
    ```
    This runs the standard `cargo test` suite for `std`-compatible crates.

*   **Embedded Target Checks:** The CI validates the build for `no_std` and embedded targets.
    ```bash
    # Check no_std build
    ./scripts/ci-check.sh no-std

    # Check embedded targets (e.g., thumbv6m-none-eabi)
    ./scripts/ci-check.sh embedded
    ```

*   **Code Coverage:**
    ```bash
    ./coverage.sh
    ```
    This script uses `grcov` to generate a coverage report.

## Development Conventions

*   **Formatting:** Code is formatted with `cargo +nightly fmt`.
*   **Source Policy:** The project enforces strict rules about file and module structure:
    *   `mod.rs` files are generally disallowed.
    *   Each file should contain only one top-level type or trait.
    *   Inherent `impl` blocks must be in the same file as their `struct` definition.
    *   Modules should re-export their descendants to create a clean public API (`check-package-exposure.rs`).
*   **Linting:** All code must pass `cargo clippy` and a suite of custom lints defined in the `lints/` directory. These are run via the `./scripts/ci-check.sh lint` command.
*   **Commit Messages:** While not explicitly documented, reviewing the git history (`git log`) is recommended to match the existing style.
*   **Dependencies:** Dependencies are sorted using the `scripts/sort_dependencies.rs` script, which can be run with `cargo make sort-dependencies`.

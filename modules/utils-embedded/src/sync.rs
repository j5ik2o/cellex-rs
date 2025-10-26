//! Foundation module for synchronization primitives.
//!
//! This module provides implementations of shared references and state cells.
//! These are used as the foundation for collections and concurrency primitives.
//!
//! # Provided Types
//!
//! - **RcShared / ArcShared**: Shared reference wrapper (implements `Shared` trait)
//! - **RcStateCell / ArcStateCell**: State cell (implements `StateCell` trait)
//!
//! # Feature Flags
//!
//! - **`rc`**: `Rc`-based implementation (single-threaded only)
//! - **`arc`**: `Arc`-based implementation (multi-threaded support)
//!   - `ArcLocal*`: Optimized implementation using local mutex
//!   - `ArcCs*`: Critical section-based implementation
//!   - `Arc*`: Standard implementation

/// `Arc`-based shared state implementations.
#[cfg(feature = "arc")]
pub mod arc;
/// `Rc`-based shared state implementations.
#[cfg(feature = "rc")]
pub mod rc;

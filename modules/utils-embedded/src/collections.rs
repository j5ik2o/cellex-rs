//! Collection types module
//!
//! This module provides collection types such as queues and stacks that are usable in `no_std`
//! environments.

/// Queue collections
///
/// Provides MPSC, priority-based, and ring buffer-based queue implementations.
pub mod queue;

/// Stack collections
///
/// Provides LIFO (Last-In-First-Out) stack implementations.
pub mod stack;

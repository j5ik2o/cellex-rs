//! Queue implementation module

/// MPSC (Multiple Producer, Single Consumer) queue
///
/// Queue implementation that supports multiple producers and a single consumer.
pub mod mpsc;

/// Priority queue
///
/// Queue implementation that controls processing order based on message priority.
pub mod priority;

/// Ring buffer queue
///
/// Efficient FIFO queue implementation using circular buffers.
pub mod ring;

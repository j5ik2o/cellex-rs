//! no_std-friendly queue primitives shared between runtimes.

mod queue_error;
mod queue_size;
mod storage;
mod traits;

mod mpsc;
mod priority;
/// Ring-buffer-based queue implementations and utilities.
pub mod ring;

pub use mpsc::{MpscBackend, MpscBuffer, MpscHandle, MpscQueue, RingBufferBackend};
pub use priority::{PriorityMessage, PriorityQueue, DEFAULT_PRIORITY, PRIORITY_LEVELS};
pub use queue_error::QueueError;
pub use queue_size::QueueSize;
#[allow(unused_imports)]
pub use ring::{RingBackend, RingBuffer, RingHandle, RingQueue, RingStorageBackend, DEFAULT_CAPACITY};
pub use storage::{QueueStorage, RingBufferStorage};
pub use traits::{QueueBase, QueueHandle as QueueRwHandle, QueueHandle, QueueReader, QueueRw, QueueWriter};

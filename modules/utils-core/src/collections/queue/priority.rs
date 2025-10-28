mod priority_message;

pub use priority_message::PriorityMessage;

/// Number of priority queue levels
///
/// By default, supports 8 priority levels.
/// Ranges from 0 (lowest priority) to 7 (highest priority).
pub const PRIORITY_LEVELS: usize = 8;

/// Default priority level
///
/// Used when message priority is not specified.
/// Defaults to the midpoint of PRIORITY_LEVELS (4).
pub const DEFAULT_PRIORITY: i8 = (PRIORITY_LEVELS / 2) as i8;

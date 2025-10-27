//! SignalKey - External wake-up signal identification

/// Signal key for external wake-up
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SignalKey(pub u64);

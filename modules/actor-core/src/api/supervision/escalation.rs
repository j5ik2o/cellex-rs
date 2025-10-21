mod root_escalation_sink;
#[cfg(all(test, feature = "std"))]
mod tests;

pub use root_escalation_sink::RootEscalationSink;

pub use crate::shared::supervision::escalation_sink::{EscalationSink, FailureEventHandler};

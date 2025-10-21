mod escalation_sink;
mod root_escalation_sink;
#[cfg(all(test, feature = "std"))]
mod tests;

pub use escalation_sink::{EscalationSink, FailureEventHandler};
pub use root_escalation_sink::RootEscalationSink;

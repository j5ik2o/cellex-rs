mod escalation_sink;
mod root_escalation_sink;
#[cfg(all(test, feature = "std"))]
mod tests;

pub use escalation_sink::EscalationSink;
pub use escalation_sink::FailureEventHandler;
pub use escalation_sink::FailureEventListener;
pub use root_escalation_sink::RootEscalationSink;

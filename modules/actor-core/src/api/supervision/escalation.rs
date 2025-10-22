mod root_escalation_sink;
#[cfg(all(test, feature = "test-support"))]
mod tests;

pub use root_escalation_sink::RootEscalationSink;

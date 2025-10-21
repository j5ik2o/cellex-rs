/// Escalation stage describing how far a failure has propagated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FailureEscalationStage {
  /// Initial failure point.
  #[default]
  Initial,
  /// Propagating towards parent.
  Escalated {
    /// Number of propagations
    hops: u8,
  },
}

impl FailureEscalationStage {
  /// Returns the initial stage.
  #[must_use]
  pub const fn initial() -> Self {
    FailureEscalationStage::Initial
  }

  /// Returns the number of escalation propagations.
  #[must_use]
  pub const fn hops(self) -> u8 {
    match self {
      | FailureEscalationStage::Initial => 0,
      | FailureEscalationStage::Escalated { hops } => hops,
    }
  }

  /// Checks if this is the initial stage.
  #[must_use]
  pub const fn is_initial(self) -> bool {
    matches!(self, FailureEscalationStage::Initial)
  }

  /// Returns the next escalation stage.
  #[must_use]
  pub const fn escalate(self) -> Self {
    match self {
      | FailureEscalationStage::Initial => FailureEscalationStage::Escalated { hops: 1 },
      | FailureEscalationStage::Escalated { hops } => {
        let next = hops.saturating_add(1);
        FailureEscalationStage::Escalated { hops: next }
      },
    }
  }
}

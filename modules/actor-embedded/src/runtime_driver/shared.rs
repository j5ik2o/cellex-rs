use alloc::vec::Vec;

use cellex_actor_core_rs::api::supervision::escalation::FailureEventListener;

/// Shared state for the embedded failure event hub.
#[derive(Default)]
pub(super) struct EmbeddedFailureEventHubState {
  pub(super) next_id:   u64,
  pub(super) listeners: Vec<(u64, FailureEventListener)>,
}

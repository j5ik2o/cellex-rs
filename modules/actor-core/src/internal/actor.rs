mod actor_cell;
mod internal_props;

pub(crate) use actor_cell::ActorCell;
pub(crate) use internal_props::InternalProps;

/// Backwards-compatible alias pointing to the API-level mailbox actor reference.
pub type InternalActorRef<M, R> = crate::api::actor::actor_ref::PriorityActorRef<M, R>;

//! Marker trait describing scheduler factories that must be shareable on multi-threaded targets.
//!
//! Internally this relies on [`cellex_utils_core_rs::sync::SharedBound`], which collapses to
//! `Send + Sync` on pointer-atomic platforms and imposes no additional bound on single-threaded
//! targets.

use cellex_utils_core_rs::sync::SharedBound;

/// Factory objects must implement this trait to be accepted by the scheduler.
pub trait ActorSchedulerFactoryBound: SharedBound {}

impl<T> ActorSchedulerFactoryBound for T where T: SharedBound {}

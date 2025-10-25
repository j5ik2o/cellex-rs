#[cfg(not(feature = "queue-v2"))]
use cellex_utils_core_rs::MpscQueue;

#[cfg(not(feature = "queue-v2"))]
use crate::api::test_support::shared_backend_handle::SharedBackendHandle;
#[cfg(feature = "queue-v2")]
use crate::shared::mailbox::queue_rw_compat::QueueRwCompat;

/// Queue abstraction backed by the shared ring-buffer handle used in tests.
#[cfg(not(feature = "queue-v2"))]
pub type TestQueue<M> = MpscQueue<SharedBackendHandle<M>, M>;

/// Queue abstraction backed by v2 collections when the `queue-v2` feature is enabled.
#[cfg(feature = "queue-v2")]
pub type TestQueue<M> = QueueRwCompat<M>;

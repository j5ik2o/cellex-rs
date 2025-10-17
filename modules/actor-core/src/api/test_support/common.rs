use crate::api::test_support::shared_backend_handle::SharedBackendHandle;
use cellex_utils_core_rs::MpscQueue;

/// Queue abstraction backed by the shared ring-buffer handle used in tests.
pub type TestQueue<M> = MpscQueue<SharedBackendHandle<M>, M>;

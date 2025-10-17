use crate::internal::mailbox::test_support::shared_backend_handle::SharedBackendHandle;
use cellex_utils_core_rs::MpscQueue;

pub type TestQueue<M> = MpscQueue<SharedBackendHandle<M>, M>;

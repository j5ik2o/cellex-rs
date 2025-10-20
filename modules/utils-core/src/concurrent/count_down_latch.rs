//! Count-down latch primitives.

pub mod count_down_latch_backend;
pub mod count_down_latch_struct;

pub use count_down_latch_backend::CountDownLatchBackend;
pub use count_down_latch_struct::CountDownLatch;

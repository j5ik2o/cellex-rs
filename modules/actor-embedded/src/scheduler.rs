#![cfg(feature = "embassy_executor")]

mod embassy_scheduler_impl;
mod runtime_ext;

pub use embassy_scheduler_impl::EmbassyScheduler;
pub use runtime_ext::{embassy_scheduler_builder, EmbassyActorRuntimeExt};

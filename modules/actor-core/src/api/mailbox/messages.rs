//! Priority-aware message utilities shared across the public API.

mod priority_channel;
mod system_message;

pub use priority_channel::PriorityChannel;
pub use system_message::SystemMessage;

#[cfg(target_has_atomic = "ptr")]
const fn assert_send_sys<T: Send>() {}

#[cfg(target_has_atomic = "ptr")]
const fn assert_sync_sys<T: Sync>() {}

#[cfg(target_has_atomic = "ptr")]
const _: () = {
  assert_send_sys::<SystemMessage>();
  assert_sync_sys::<SystemMessage>();
  assert_static_sys::<SystemMessage>();
};

#[cfg(target_has_atomic = "ptr")]
const fn assert_static_sys<T: 'static>() {}

use core::cell::UnsafeCell;

use cellex_utils_core_rs::QueueError;
use portable_atomic::{AtomicU8, Ordering};

use crate::{api::mailbox::messages::PriorityEnvelope, shared::messaging::AnyMessage};

#[cfg(not(target_has_atomic = "ptr"))]
mod local_waker {
  use core::{cell::RefCell, task::Waker};

  /// Single-threaded waker used on targets without atomic pointer support.
  #[derive(Default)]
  pub struct LocalWaker {
    stored: RefCell<Option<Waker>>,
  }

  impl LocalWaker {
    pub fn new() -> Self {
      Self::default()
    }

    pub fn register(&self, waker: &Waker) {
      *self.stored.borrow_mut() = Some(waker.clone());
    }

    pub fn wake(&self) {
      if let Some(waker) = self.stored.borrow_mut().take() {
        waker.wake();
      }
    }
  }
}

#[cfg(target_has_atomic = "ptr")]
pub(crate) type SharedWaker = futures::task::AtomicWaker;

#[cfg(not(target_has_atomic = "ptr"))]
pub(crate) type SharedWaker = local_waker::LocalWaker;

#[cfg(target_has_atomic = "ptr")]
pub(crate) type DispatchFn =
  dyn Fn(AnyMessage, i8) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>> + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
pub(crate) type DispatchFn = dyn Fn(AnyMessage, i8) -> Result<(), QueueError<PriorityEnvelope<AnyMessage>>>;

#[cfg(target_has_atomic = "ptr")]
pub(crate) type DropHookFn = dyn Fn() + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
pub(crate) type DropHookFn = dyn Fn();

pub(crate) const STATE_PENDING: u8 = 0;
pub(crate) const STATE_READY: u8 = 1;
pub(crate) const STATE_CANCELLED: u8 = 2;
pub(crate) const STATE_RESPONDER_DROPPED: u8 = 3;

/// Internal state shared between `AskFuture` and responder.
pub(crate) struct AskShared<Resp> {
  pub(crate) state: AtomicU8,
  pub(crate) value: UnsafeCell<Option<Resp>>,
  pub(crate) waker: SharedWaker,
}

impl<Resp> AskShared<Resp> {
  #[cfg(target_has_atomic = "ptr")]
  pub(crate) const fn new() -> Self {
    Self { state: AtomicU8::new(STATE_PENDING), value: UnsafeCell::new(None), waker: SharedWaker::new() }
  }

  #[cfg(not(target_has_atomic = "ptr"))]
  pub(crate) fn new() -> Self {
    Self { state: AtomicU8::new(STATE_PENDING), value: UnsafeCell::new(None), waker: SharedWaker::new() }
  }

  pub(crate) fn complete(&self, value: Resp) -> bool {
    match self.state.compare_exchange(STATE_PENDING, STATE_READY, Ordering::AcqRel, Ordering::Acquire) {
      | Ok(_) => {
        unsafe {
          *self.value.get() = Some(value);
        }
        self.waker.wake();
        true
      },
      | Err(_) => false,
    }
  }

  pub(crate) fn cancel(&self) -> bool {
    self
      .state
      .compare_exchange(STATE_PENDING, STATE_CANCELLED, Ordering::AcqRel, Ordering::Acquire)
      .map(|_| {
        self.waker.wake();
      })
      .is_ok()
  }

  pub(crate) fn responder_dropped(&self) {
    if self.state.compare_exchange(STATE_PENDING, STATE_RESPONDER_DROPPED, Ordering::AcqRel, Ordering::Acquire).is_ok()
    {
      self.waker.wake();
    }
  }

  pub(crate) unsafe fn take_value(&self) -> Option<Resp> {
    unsafe { (*self.value.get()).take() }
  }

  pub(crate) fn state(&self) -> u8 {
    self.state.load(Ordering::Acquire)
  }
}

unsafe impl<Resp: Send> Send for AskShared<Resp> {}
unsafe impl<Resp: Send> Sync for AskShared<Resp> {}

use crate::api::mailbox::PriorityEnvelope;
use crate::DynMessage;
use cellex_utils_core_rs::QueueError;
use core::cell::UnsafeCell;
use portable_atomic::{AtomicU8, Ordering};

#[cfg(not(target_has_atomic = "ptr"))]
mod local_waker {
  use core::cell::RefCell;
  use core::task::Waker;

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
pub(super) type SharedWaker = futures::task::AtomicWaker;

#[cfg(not(target_has_atomic = "ptr"))]
pub(super) type SharedWaker = local_waker::LocalWaker;

#[cfg(target_has_atomic = "ptr")]
pub(super) type DispatchFn =
  dyn Fn(DynMessage, i8) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>> + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
pub(super) type DispatchFn = dyn Fn(DynMessage, i8) -> Result<(), QueueError<PriorityEnvelope<DynMessage>>>;

#[cfg(target_has_atomic = "ptr")]
pub(super) type DropHookFn = dyn Fn() + Send + Sync;

#[cfg(not(target_has_atomic = "ptr"))]
pub(super) type DropHookFn = dyn Fn();

pub(super) const STATE_PENDING: u8 = 0;
pub(super) const STATE_READY: u8 = 1;
pub(super) const STATE_CANCELLED: u8 = 2;
pub(super) const STATE_RESPONDER_DROPPED: u8 = 3;

/// Internal state shared between `AskFuture` and responder.
pub(super) struct AskShared<Resp> {
  pub(super) state: AtomicU8,
  pub(super) value: UnsafeCell<Option<Resp>>,
  pub(super) waker: SharedWaker,
}

impl<Resp> AskShared<Resp> {
  #[cfg(target_has_atomic = "ptr")]
  pub(super) const fn new() -> Self {
    Self {
      state: AtomicU8::new(STATE_PENDING),
      value: UnsafeCell::new(None),
      waker: SharedWaker::new(),
    }
  }

  #[cfg(not(target_has_atomic = "ptr"))]
  pub(super) fn new() -> Self {
    Self {
      state: AtomicU8::new(STATE_PENDING),
      value: UnsafeCell::new(None),
      waker: SharedWaker::new(),
    }
  }

  pub(super) fn complete(&self, value: Resp) -> bool {
    match self
      .state
      .compare_exchange(STATE_PENDING, STATE_READY, Ordering::AcqRel, Ordering::Acquire)
    {
      Ok(_) => {
        unsafe {
          *self.value.get() = Some(value);
        }
        self.waker.wake();
        true
      }
      Err(_) => false,
    }
  }

  pub(super) fn cancel(&self) -> bool {
    self
      .state
      .compare_exchange(STATE_PENDING, STATE_CANCELLED, Ordering::AcqRel, Ordering::Acquire)
      .map(|_| {
        self.waker.wake();
      })
      .is_ok()
  }

  pub(super) fn responder_dropped(&self) {
    if self
      .state
      .compare_exchange(
        STATE_PENDING,
        STATE_RESPONDER_DROPPED,
        Ordering::AcqRel,
        Ordering::Acquire,
      )
      .is_ok()
    {
      self.waker.wake();
    }
  }

  pub(super) unsafe fn take_value(&self) -> Option<Resp> {
    unsafe { (*self.value.get()).take() }
  }

  pub(super) fn state(&self) -> u8 {
    self.state.load(Ordering::Acquire)
  }
}

unsafe impl<Resp: Send> Send for AskShared<Resp> {}
unsafe impl<Resp: Send> Sync for AskShared<Resp> {}

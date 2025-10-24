use core::task::{Context, Poll, Waker};

use portable_atomic::{AtomicU8, Ordering};
use spin::Mutex;

const STATE_PENDING: u8 = 0;
const STATE_COMPLETED: u8 = 1;
const STATE_CANCELLED: u8 = 2;

/// Internal node representing a single waiter.
pub struct WaitNode<E> {
  state:  AtomicU8,
  waker:  Mutex<Option<Waker>>,
  result: Mutex<Option<Result<(), E>>>,
}

impl<E: Copy> WaitNode<E> {
  /// Creates a new pending waiter node.
  pub const fn new() -> Self {
    Self { state: AtomicU8::new(STATE_PENDING), waker: Mutex::new(None), result: Mutex::new(None) }
  }

  /// Completes the waiter with the provided result.
  pub fn complete(&self, value: Result<(), E>) -> bool {
    let mut result_guard = self.result.lock();
    if self.state.compare_exchange(STATE_PENDING, STATE_COMPLETED, Ordering::AcqRel, Ordering::Acquire).is_err() {
      return false;
    }

    *result_guard = Some(value);
    drop(result_guard);

    if let Some(waker) = self.waker.lock().take() {
      waker.wake();
    }

    true
  }

  /// Marks the waiter as cancelled.
  pub fn cancel(&self) {
    if self.state.swap(STATE_CANCELLED, Ordering::AcqRel) == STATE_PENDING {
      self.waker.lock().take();
    }
  }

  /// Polls the waiter for completion.
  pub fn poll(&self, cx: &mut Context<'_>) -> Poll<Result<(), E>> {
    match self.state.load(Ordering::Acquire) {
      | STATE_COMPLETED => {
        let guard = self.result.lock();
        let value = match *guard {
          | Some(result) => result,
          | None => panic!("completed waiter must hold a result"),
        };
        Poll::Ready(value)
      },
      | STATE_CANCELLED => Poll::Pending,
      | _ => {
        *self.waker.lock() = Some(cx.waker().clone());

        if self.state.load(Ordering::Acquire) == STATE_COMPLETED {
          let guard = self.result.lock();
          let value = match *guard {
            | Some(result) => result,
            | None => panic!("completed waiter must hold a result"),
          };
          Poll::Ready(value)
        } else {
          Poll::Pending
        }
      },
    }
  }

  /// Completes the waiter with the specified error.
  pub fn complete_with_error(&self, error: E) {
    let _ = self.complete(Err(error));
  }

  /// Completes the waiter successfully.
  pub fn complete_ok(&self) -> bool {
    self.complete(Ok(()))
  }
}

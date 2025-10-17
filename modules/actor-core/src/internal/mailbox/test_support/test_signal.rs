use crate::api::mailbox::MailboxSignal;
use crate::internal::mailbox::test_support::test_signal_state::TestSignalState;
use crate::internal::mailbox::test_support::test_signal_wait::TestSignalWait;
use cellex_utils_core_rs::ArcShared;
use core::cell::RefCell;
use core::marker::PhantomData;

#[derive(Clone)]
pub struct TestSignal {
  pub(crate) state: ArcShared<RefCell<TestSignalState>>,
}

impl Default for TestSignal {
  fn default() -> Self {
    Self {
      state: ArcShared::new(RefCell::new(TestSignalState::default())),
    }
  }
}

impl MailboxSignal for TestSignal {
  type WaitFuture<'a>
    = TestSignalWait<'a>
  where
    Self: 'a;

  fn notify(&self) {
    let mut state = self.state.borrow_mut();
    state.notified = true;
    if let Some(waker) = state.waker.take() {
      waker.wake();
    }
  }

  fn wait(&self) -> Self::WaitFuture<'_> {
    TestSignalWait {
      signal: self.clone(),
      _marker: PhantomData,
    }
  }
}

use cellex_utils_core_rs::v2::collections::queue::QueueError as AsyncQueueError;
use cellex_utils_std_rs::v2::collections::make_tokio_mpsc_queue;

#[tokio::test(flavor = "multi_thread")]
async fn async_queue_roundtrip() {
  let queue = make_tokio_mpsc_queue::<usize>(8);
  let (producer, consumer) = queue.clone().into_mpsc_pair();

  producer.offer(10).await.expect("offer should succeed");
  let len: usize = queue.len().await.expect("len");
  assert_eq!(len, 1);
  let capacity: usize = queue.capacity().await.expect("capacity");
  assert_eq!(capacity, 8);

  let received = consumer.poll().await.expect("poll result");
  assert_eq!(received, 10);
  let empty = queue.is_empty().await.expect("is_empty");
  assert!(empty);
}

#[tokio::test(flavor = "multi_thread")]
async fn async_queue_blocking_behavior() {
  let queue = make_tokio_mpsc_queue::<u8>(1);
  let (producer, consumer) = queue.clone().into_mpsc_pair();

  producer.offer(1).await.expect("first offer");

  let pending = producer.offer(2);
  tokio::pin!(pending);
  tokio::select! {
    _ = &mut pending => panic!("second offer should await capacity"),
    _ = tokio::time::sleep(std::time::Duration::from_millis(20)) => {}
  }

  assert_eq!(consumer.poll().await.expect("consume first"), 1);
  pending.await.expect("second offer eventually succeeds");
  assert_eq!(consumer.poll().await.expect("consume second"), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn async_queue_would_block_on_full_policy() {
  use cellex_utils_core_rs::{
    sync::{async_mutex_like::SpinAsyncMutex, interrupt::InterruptContextPolicy},
    v2::{
      collections::queue::{
        backend::{OverflowPolicy, SyncAdapterQueueBackend, VecRingBackend},
        type_keys::MpscKey,
        AsyncQueue,
      },
      sync::SharedError,
    },
    ArcShared,
  };

  struct DenyPolicy;
  impl InterruptContextPolicy for DenyPolicy {
    fn check_blocking_allowed() -> Result<(), SharedError> {
      Err(SharedError::InterruptContext)
    }
  }

  type DenyMutex<T> = SpinAsyncMutex<T, DenyPolicy>;

  let storage = cellex_utils_core_rs::v2::collections::queue::VecRingStorage::with_capacity(1);
  let backend = VecRingBackend::new_with_storage(storage, OverflowPolicy::Block);
  let shared = ArcShared::new(DenyMutex::new(SyncAdapterQueueBackend::new(backend)));
  let queue: AsyncQueue<i32, MpscKey, _, _> = AsyncQueue::new_mpsc(shared);
  let (producer, consumer) = queue.into_mpsc_pair();

  let err = producer.offer(1).await.unwrap_err();
  assert!(matches!(err, AsyncQueueError::WouldBlock));
  let err = consumer.poll().await.unwrap_err();
  assert!(matches!(err, AsyncQueueError::WouldBlock));
}

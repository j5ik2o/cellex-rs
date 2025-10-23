use std::{cell::RefCell, rc::Rc};

use cellex_utils_core_rs::{
  collections::stack::{
    buffer::StackBuffer,
    traits::{StackHandle, StackStorage, StackStorageBackend},
    Stack as LegacyStack,
  },
  sync::{sync_mutex_like::SpinSyncMutex, ArcShared, Shared},
  v2::collections::stack::{
    backend::{StackOverflowPolicy, VecStackBackend},
    SharedVecStack, VecStackStorage,
  },
};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

type LegacyBackend<T> = StackStorageBackend<RcStorageHandle<T>>;
type LegacyStackHandle<T> = Rc<LegacyBackend<T>>;

struct RcStorageHandle<T>(Rc<RefCell<StackBuffer<T>>>);

impl<T> Clone for RcStorageHandle<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> core::ops::Deref for RcStorageHandle<T> {
  type Target = RefCell<StackBuffer<T>>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T> StackStorage<T> for RcStorageHandle<T> {
  fn with_read<R>(&self, f: impl FnOnce(&StackBuffer<T>) -> R) -> R {
    f(&self.borrow())
  }

  fn with_write<R>(&self, f: impl FnOnce(&mut StackBuffer<T>) -> R) -> R {
    f(&mut self.borrow_mut())
  }
}

impl<T> Shared<RefCell<StackBuffer<T>>> for RcStorageHandle<T> {}

struct RcBackendHandle<T>(LegacyStackHandle<T>);

impl<T> Clone for RcBackendHandle<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> core::ops::Deref for RcBackendHandle<T> {
  type Target = LegacyBackend<T>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T> Shared<LegacyBackend<T>> for RcBackendHandle<T> {}

impl<T> StackHandle<T> for RcBackendHandle<T> {
  type Backend = LegacyBackend<T>;

  fn backend(&self) -> &Self::Backend {
    &self.0
  }
}

fn make_legacy_stack(capacity: usize) -> LegacyStack<RcBackendHandle<u32>, u32> {
  let storage = RcStorageHandle(Rc::new(RefCell::new(StackBuffer::new())));
  let backend = RcBackendHandle(Rc::new(LegacyBackend::new(storage)));
  let stack = LegacyStack::new(backend);
  stack.set_capacity(Some(capacity));
  stack
}

fn make_v2_stack(capacity: usize) -> SharedVecStack<u32> {
  let storage = VecStackStorage::with_capacity(capacity);
  let backend = VecStackBackend::new_with_storage(storage, StackOverflowPolicy::Grow);
  let shared = ArcShared::new(SpinSyncMutex::new(backend));
  SharedVecStack::new(shared)
}

fn bench_stack_push_pop(c: &mut Criterion) {
  let mut group = c.benchmark_group("stack_push_pop");
  let batch = 256_u32;

  group.bench_function("legacy_rc_stack", |b| {
    b.iter_batched(
      || make_legacy_stack(batch as usize),
      |stack| {
        for value in 0..batch {
          stack.push(value).unwrap();
        }
        for _ in 0..batch {
          let _ = stack.pop();
        }
      },
      BatchSize::SmallInput,
    );
  });

  group.bench_function("v2_vec_stack", |b| {
    b.iter_batched(
      || make_v2_stack(batch as usize),
      |stack| {
        for value in 0..batch {
          stack.push(value).unwrap();
        }
        for _ in 0..batch {
          let _ = stack.pop().unwrap();
        }
      },
      BatchSize::SmallInput,
    );
  });

  group.finish();
}

criterion_group!(benches, bench_stack_push_pop);
criterion_main!(benches);

# 同期プリミティブの抽象化設計

## 現状の問題

### `spin::Mutex` の広範な使用

**使用箇所**(6箇所):
1. `internal/scheduler/ready_queue_scheduler/ready_queue_scheduler.rs`
2. `internal/scheduler/ready_queue_scheduler/ready_queue_worker_impl.rs`
3. `internal/scheduler/ready_queue_scheduler/ready_queue_context.rs`
4. `internal/scheduler/ready_queue_scheduler/ready_notifier.rs`
5. `internal/guardian/tests.rs`
6. `api/actor/props.rs`

**具体例**:
```rust
// ready_queue_scheduler.rs:41-42
pub struct ReadyQueueScheduler<M, MF, Strat> {
  context: ArcShared<Mutex<ReadyQueueContext<M, MF, Strat>>>,  // spin::Mutex
  state: ArcShared<Mutex<ReadyQueueState>>,                     // spin::Mutex
}

// props.rs:56
let handler_cell = ArcShared::new(Mutex::new(handler));  // spin::Mutex
```

### ランタイム環境ごとの適性

| 環境 | 適切なMutex | 理由 |
|------|------------|------|
| **組み込み(no_std)** | `spin::Mutex` | OSなし、割り込みコンテキスト対応 |
| **Tokio(async std)** | `tokio::sync::Mutex` | `.await`可能、協調的スケジューリング |
| **標準(sync std)** | `std::sync::Mutex` | OS最適化、効率的なブロック |

### 現状の評価

#### ✅ 良い点
- **組み込み環境で動作**: `spin::Mutex`はno_std対応
- **ロックスコープが短い**: `.await`の前に必ずロック解放
- **シンプル**: 追加の依存なし

#### ⚠️ 潜在的リスク
- **Tokio環境で非効率**: スピンロックでCPU消費
- **将来的なバグリスク**: ロック保持したまま`.await`するとデッドロック
- **ランタイム非依存**: 各環境で最適化されていない

## 設計方針

### 原則
1. **ランタイム抽象化**: 環境ごとに最適なMutexを使用
2. **後方互換性**: 既存のAPIを破壊しない
3. **段階的移行**: 3つのフェーズで実装
4. **パフォーマンス優先**: ゼロコスト抽象化

## 設計案

### Option 1: Feature Flag による条件コンパイル(推奨)

#### 実装

```rust
// shared/sync_primitives.rs (新規作成)

/// Runtime-optimized mutex abstraction.
///
/// Automatically selects the best mutex implementation based on the target environment:
/// - Tokio runtime: `tokio::sync::Mutex` (cooperative, async-aware)
/// - Standard runtime: `std::sync::Mutex` (OS-optimized blocking)
/// - Embedded/no_std: `spin::Mutex` (lock-free, interrupt-safe)
#[cfg(all(feature = "std", feature = "tokio-runtime"))]
pub type RuntimeMutex<T> = tokio::sync::Mutex<T>;

#[cfg(all(feature = "std", not(feature = "tokio-runtime")))]
pub type RuntimeMutex<T> = std::sync::Mutex<T>;

#[cfg(not(feature = "std"))]
pub type RuntimeMutex<T> = spin::Mutex<T>;

/// Runtime-optimized RwLock abstraction.
#[cfg(all(feature = "std", feature = "tokio-runtime"))]
pub type RuntimeRwLock<T> = tokio::sync::RwLock<T>;

#[cfg(all(feature = "std", not(feature = "tokio-runtime")))]
pub type RuntimeRwLock<T> = std::sync::RwLock<T>;

#[cfg(not(feature = "std"))]
pub type RuntimeRwLock<T> = spin::RwLock<T>;
```

#### Cargo.toml 設定

```toml
[features]
default = ["std"]
std = []
tokio-runtime = ["tokio/sync", "std"]
embedded = []  # no_std

[dependencies]
tokio = { version = "1", features = ["sync"], optional = true }
spin = { version = "0.9", default-features = false }
```

#### 使用例

```rust
// ready_queue_scheduler.rs
use crate::shared::sync_primitives::RuntimeMutex;

pub struct ReadyQueueScheduler<M, MF, Strat> {
  context: ArcShared<RuntimeMutex<ReadyQueueContext<M, MF, Strat>>>,
  state: ArcShared<RuntimeMutex<ReadyQueueState>>,
}
```

#### メリット
- ✅ シンプルで理解しやすい
- ✅ コンパイル時に最適化される(ゼロコストabstraction)
- ✅ 既存コードの変更が最小限
- ✅ 後方互換性維持

#### デメリット
- ⚠️ `tokio::Mutex`は`.await`が必要(APIが変わる)
- ⚠️ async/sync混在コードでの扱いが複雑

### Option 2: Trait ベースの抽象化

#### 実装

```rust
// shared/sync_primitives.rs

/// Generic mutex abstraction for runtime-agnostic code.
pub trait MutexLike<T> {
    type Guard<'a>: DerefMut<Target = T> where Self: 'a, T: 'a;

    fn lock(&self) -> Self::Guard<'_>;
}

/// Async-aware mutex abstraction.
pub trait AsyncMutexLike<T> {
    type Guard<'a>: DerefMut<Target = T> where Self: 'a, T: 'a;

    async fn lock(&self) -> Self::Guard<'_>;
}

// spin::Mutex implementation
impl<T> MutexLike<T> for spin::Mutex<T> {
    type Guard<'a> = spin::MutexGuard<'a, T> where T: 'a;

    fn lock(&self) -> Self::Guard<'_> {
        self.lock()
    }
}

// tokio::Mutex implementation
#[cfg(feature = "tokio-runtime")]
impl<T> AsyncMutexLike<T> for tokio::sync::Mutex<T> {
    type Guard<'a> = tokio::sync::MutexGuard<'a, T> where T: 'a;

    async fn lock(&self) -> Self::Guard<'_> {
        self.lock().await
    }
}
```

#### メリット
- ✅ 柔軟性が高い
- ✅ 複数のMutex実装を同時に使える
- ✅ テストでモックMutexを注入可能

#### デメリット
- ❌ 複雑すぎる(over-engineering)
- ❌ GATs(Generic Associated Types)が必要
- ❌ パフォーマンスオーバーヘッド

### Option 3: MailboxFactory 経由の抽象化

#### 実装

```rust
// api/mailbox/mailbox_factory.rs

pub trait MailboxFactory {
    type Mutex<T>: MutexLike<T>;  // Associated type
    type RwLock<T>: RwLockLike<T>;

    // ... existing associated types
}

// Tokio実装
impl MailboxFactory for TokioMailboxFactory {
    type Mutex<T> = tokio::sync::Mutex<T>;
    type RwLock<T> = tokio::sync::RwLock<T>;
    // ...
}

// Embedded実装
impl MailboxFactory for EmbeddedMailboxFactory {
    type Mutex<T> = spin::Mutex<T>;
    type RwLock<T> = spin::RwLock<T>;
    // ...
}
```

#### ReadyQueueScheduler への適用

```rust
pub struct ReadyQueueScheduler<M, MF, Strat>
where
  MF: MailboxFactory,
{
  context: ArcShared<MF::Mutex<ReadyQueueContext<M, MF, Strat>>>,
  state: ArcShared<MF::Mutex<ReadyQueueState>>,
}
```

#### メリット
- ✅ ランタイムとの統合が自然
- ✅ 既存のMailboxFactory抽象化に沿っている
- ✅ 型安全性が高い

#### デメリット
- ❌ `Props`など非scheduler箇所で使いにくい
- ❌ MailboxFactoryの責務が大きくなりすぎる

## 推奨実装: Option 1 (Feature Flag)

### 理由
1. **シンプル**: 理解しやすく、保守しやすい
2. **パフォーマンス**: コンパイル時最適化、ゼロコスト
3. **互換性**: 既存コードの変更が最小限
4. **実績**: Rustエコシステムで広く使われているパターン

## 移行計画

### Phase 1: 抽象化レイヤーの追加(Week 1-2)

#### 1.1 `shared/sync_primitives.rs` 作成

```rust
// modules/actor-core/src/shared/sync_primitives.rs

//! Runtime-optimized synchronization primitives.
//!
//! This module provides abstractions over different mutex implementations
//! to ensure optimal performance across different runtime environments.

#[cfg(all(feature = "std", feature = "tokio-runtime"))]
pub type RuntimeMutex<T> = tokio::sync::Mutex<T>;

#[cfg(all(feature = "std", not(feature = "tokio-runtime")))]
pub type RuntimeMutex<T> = std::sync::Mutex<T>;

#[cfg(not(feature = "std"))]
pub type RuntimeMutex<T> = spin::Mutex<T>;

#[cfg(all(feature = "std", feature = "tokio-runtime"))]
pub type RuntimeRwLock<T> = tokio::sync::RwLock<T>;

#[cfg(all(feature = "std", not(feature = "tokio-runtime")))]
pub type RuntimeRwLock<T> = std::sync::RwLock<T>;

#[cfg(not(feature = "std"))]
pub type RuntimeRwLock<T> = spin::RwLock<T>;

/// Helper macro for locking with appropriate async/sync semantics.
#[cfg(all(feature = "std", feature = "tokio-runtime"))]
#[macro_export]
macro_rules! runtime_lock {
    ($mutex:expr) => {
        $mutex.lock().await
    };
}

#[cfg(not(all(feature = "std", feature = "tokio-runtime")))]
#[macro_export]
macro_rules! runtime_lock {
    ($mutex:expr) => {
        $mutex.lock()
    };
}
```

#### 1.2 `shared.rs` に追加

```rust
// modules/actor-core/src/shared.rs

pub mod failure_telemetry;
pub mod map_system;
pub mod receive_timeout;
pub mod sync_primitives;  // 追加

pub use sync_primitives::*;
```

#### 1.3 Cargo.toml 更新

```toml
# modules/actor-core/Cargo.toml

[features]
default = ["std"]
std = []
tokio-runtime = ["tokio/sync", "std"]

[dependencies]
tokio = { version = "1", features = ["sync"], optional = true }
spin = "0.9"
```

### Phase 2: 段階的な移行(Week 3-4)

#### 2.1 ReadyQueueScheduler の移行

```rust
// ready_queue_scheduler.rs

// Before
use spin::Mutex;

// After
use crate::shared::sync_primitives::RuntimeMutex;

pub struct ReadyQueueScheduler<M, MF, Strat> {
  context: ArcShared<RuntimeMutex<ReadyQueueContext<M, MF, Strat>>>,
  state: ArcShared<RuntimeMutex<ReadyQueueState>>,
}
```

**注意**: `tokio-runtime` feature有効時は`.lock().await`が必要

#### 2.2 Props の移行

```rust
// props.rs

// Before
use spin::Mutex;
let handler_cell = ArcShared::new(Mutex::new(handler));

// After
use crate::shared::sync_primitives::RuntimeMutex;
let handler_cell = ArcShared::new(RuntimeMutex::new(handler));
```

#### 2.3 その他のファイル

- `ready_queue_worker_impl.rs`
- `ready_queue_context.rs`
- `ready_notifier.rs`

### Phase 3: テストと検証(Week 5-6)

#### 3.1 各環境でのテスト

```bash
# 組み込み環境(no_std)
cargo test --no-default-features --features embedded

# 標準環境(std)
cargo test --features std

# Tokio環境
cargo test --features tokio-runtime
```

#### 3.2 パフォーマンスベンチマーク

```rust
// benches/mutex_comparison.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_spin_mutex(c: &mut Criterion) {
    c.bench_function("spin_mutex", |b| {
        let mutex = spin::Mutex::new(0);
        b.iter(|| {
            let mut guard = mutex.lock();
            *guard += 1;
            black_box(*guard);
        });
    });
}

fn bench_tokio_mutex(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("tokio_mutex", |b| {
        let mutex = tokio::sync::Mutex::new(0);
        b.to_async(&rt).iter(|| async {
            let mut guard = mutex.lock().await;
            *guard += 1;
            black_box(*guard);
        });
    });
}

criterion_group!(benches, bench_spin_mutex, bench_tokio_mutex);
criterion_main!(benches);
```

#### 3.3 Clippy Lint 追加

```rust
// lib.rs に追加

#![deny(clippy::await_holding_lock)]
#![warn(clippy::mutex_atomic)]
```

### Phase 4: ドキュメント更新(Week 7)

#### 4.1 CLAUDE.md 更新

```markdown
## 同期プリミティブの使用

- **`RuntimeMutex<T>`を使用すること**: `spin::Mutex`を直接使わない
- **ロックスコープを最小化**: `.await`の前に必ずロックを解放
- **Clippy警告に従う**: `await_holding_lock`を避ける
```

#### 4.2 Migration Guide 更新

```markdown
### Synchronization Primitives Migration

**Before**:
\`\`\`rust
use spin::Mutex;
let state = Mutex::new(data);
\`\`\`

**After**:
\`\`\`rust
use crate::shared::RuntimeMutex;
let state = RuntimeMutex::new(data);
\`\`\`
```

## async/sync ハイブリッドAPIの設計

### 問題: `tokio::Mutex` は `.await` が必要

```rust
// spin::Mutex (sync)
let guard = mutex.lock();

// tokio::Mutex (async)
let guard = mutex.lock().await;  // ❌ API が変わる!
```

### 解決策1: Conditional Compilation

```rust
// ready_queue_scheduler.rs

#[cfg(not(feature = "tokio-runtime"))]
pub fn spawn_actor(&mut self, ...) -> Result<...> {
    let mut ctx = self.context.lock();
    ctx.spawn_actor(supervisor, context)
}

#[cfg(feature = "tokio-runtime")]
pub async fn spawn_actor(&mut self, ...) -> Result<...> {
    let mut ctx = self.context.lock().await;
    ctx.spawn_actor(supervisor, context)
}
```

**問題**: API が feature flag で変わる(非互換)

### 解決策2: 常に async を使う(推奨)

```rust
// すべてのメソッドを async にする
pub async fn spawn_actor(&mut self, ...) -> Result<...> {
    #[cfg(feature = "tokio-runtime")]
    let mut ctx = self.context.lock().await;

    #[cfg(not(feature = "tokio-runtime"))]
    let mut ctx = self.context.lock();

    ctx.spawn_actor(supervisor, context)
}
```

**利点**:
- ✅ APIが統一される
- ✅ 将来的な async 化に対応しやすい

**欠点**:
- ⚠️ sync環境で不要な async/await が増える(が、ゼロコストで最適化される)

### 解決策3: マクロで統一

```rust
// shared/sync_primitives.rs

#[cfg(feature = "tokio-runtime")]
#[macro_export]
macro_rules! lock {
    ($mutex:expr) => {
        $mutex.lock().await
    };
}

#[cfg(not(feature = "tokio-runtime"))]
#[macro_export]
macro_rules! lock {
    ($mutex:expr) => {
        $mutex.lock()
    };
}

// 使用例
pub async fn spawn_actor(&mut self, ...) -> Result<...> {
    let mut ctx = lock!(self.context);
    ctx.spawn_actor(supervisor, context)
}
```

## パフォーマンス影響の見積もり

### Tokio環境

| 操作 | spin::Mutex | tokio::Mutex | 差分 |
|------|-------------|--------------|------|
| lock/unlock | ~10ns | ~60ns | +50ns |
| contended lock | スピン待ち(CPU消費) | yield(CPU効率的) | 状況依存 |

**結論**: クリティカルセクションが10μs以下なら差はほぼ無視できる

### 組み込み環境

| 実装 | 適性 | 理由 |
|------|------|------|
| `spin::Mutex` | ✅ 最適 | 割り込みコンテキスト対応、OSなし |
| `std::Mutex` | ❌ 使用不可 | std依存 |
| `tokio::Mutex` | ❌ 使用不可 | Tokioランタイム必須 |

## まとめ

### 短期的対応(すぐやる)
1. ✅ `shared/sync_primitives.rs` 作成
2. ✅ Feature flags 設定
3. ✅ Clippy lint 追加

### 中期的対応(Phase 2-3)
4. ⏳ `RuntimeMutex<T>` への段階的移行
5. ⏳ 各環境でのテスト
6. ⏳ パフォーマンスベンチマーク

### 長期的対応(Phase 3+)
7. 🔄 ロックフリー設計への移行検討
8. 🔄 async/await の完全統合

**推奨**: まずPhase 1を実装し、既存の`spin::Mutex`を`RuntimeMutex`に置き換えるPRを作成する。

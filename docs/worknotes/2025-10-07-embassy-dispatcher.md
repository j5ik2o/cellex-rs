# Embassy 向けディスパッチループ統合メモ (2025-10-07)

## 目的
`cellex-actor-embedded-rs` の `define_embassy_dispatcher!` マクロを利用し、Embassy executor 上で
`ActorSystem<U, LocalMailboxFactory>` のディスパッチループを常駐させる手順をまとめる。

## 必要条件
- `cellex-actor-embedded-rs` で `embassy_executor` フィーチャを有効化する。
- `embassy-executor` / `embassy-sync` が利用側プロジェクトで初期化済みであること。
- `ActorSystem` を `'static` な領域（例: `StaticCell`）に配置できること。

## 手順
1. Cargo フィーチャを有効化する。
   ```toml
   cellex-actor-embedded-rs = { version = "*", features = ["embassy_executor"] }
   ```
2. `StaticCell` などで `ActorSystem` を確保し、`LocalMailboxFactory` など適切なランタイムを渡して初期化する。
3. `ActorRuntimeBundle::new(factory).with_embassy_scheduler()` を利用して、Embassy 用スケジューラを組み込んだ `ActorSystem::new_with_runtime` を構築する。（`target_has_atomic = "ptr"` が `false` のターゲットで利用可能）
4. `define_embassy_dispatcher!` マクロで `Spawner::spawn` に渡すタスクを定義し、初期化した `ActorSystem` の可変参照をタスクへ渡す。
5. 以降は通常どおり `root_context()` からアクターを起動すれば、Embassy タスクが自動的に `dispatch_next` を駆動する。

## サンプルコード

リポジトリには `modules/actor-embedded/examples/embassy_run_forever.rs` を追加済み。`--features embassy_executor`
でビルドすると、`StaticCell` に配置した `ActorSystem<u32, LocalMailboxFactory>` を Embassy executor 上で常駐させ、
送信したメッセージの合計値をグローバルな `AtomicU32` へ記録する最小サンプルとして動作する。

```rust
use cellex_actor_core_rs::{ActorRuntimeBundle, ActorSystem, ActorSystemConfig, MailboxOptions};
use cellex_actor_embedded_rs::{define_embassy_dispatcher, LocalMailboxFactory};
use embassy_executor::Spawner;
use static_cell::StaticCell;

static SYSTEM: StaticCell<ActorSystem<u32, LocalMailboxFactory>> = StaticCell::new();

define_embassy_dispatcher!(
  pub fn dispatcher(system: ActorSystem<u32, LocalMailboxFactory>)
);

pub fn start(spawner: &Spawner) {
  let runtime = LocalMailboxFactory::default();
  let bundle = ActorRuntimeBundle::new(runtime).with_embassy_scheduler();
  let system = SYSTEM.init_with(|| ActorSystem::new_with_runtime(bundle, ActorSystemConfig::default()));

  // Embassy タスクとしてディスパッチを起動
  spawner.spawn(dispatcher(system)).expect("spawn dispatcher");

  // 以降、RootContext を通じてアクターを起動
  let mut root = system.root_context();
  // ... root.spawn(...)
}
```

## 今後の TODO
- Embassy 用タイマー／シグナル実装を共通化し、`spawn_child` から直接 Embassy の I/O を扱えるようにする。
- `define_embassy_dispatcher!` が定義するタスク終了時の通知経路（ログ・イベントベース）の整備。

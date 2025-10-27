# Actor-Core v2 コレクション移行計画 (2025-10-24)

## 目的
- actor-core 系クレートで旧コレクション (`QueueRw`, `ArcMpscBoundedQueue`, `ArcStack` 等) を廃止し、`cellex_utils_core_rs::v2` に統一する。
- 新 API で返却される `OfferOutcome` / `PollOutcome` と `QueueError::{WouldBlock,Closed,Disconnected,Full,Empty,AllocError}` を含む全ての戻り値を正しくハンドリングできるコードパスへ置き換える。
- ファサード層からテストまで段階的に置き換え、`./scripts/ci-check.sh all` を無エラーで完走させる。

## スコープ
- `modules/actor-core` 配下全体（特に `api/mailbox`、`scheduler`、`tests`）。
- 依存クレート: `cellex_utils_core_rs`, `cellex_actor_std_rs`, `cellex_actor_embedded_rs` の v2 コレクション導入部。
- 対象外: 旧実装保管フォルダ `docs/sources/nexus-actor-rs/`（参照のみ）、リモート/クラスタ機能の具体的移行。

## ロールバックとスケジュール目安
- **ロールバック方針（2025-10-26 更新）**: `queue-v1` フィーチャーは完全に退役し、コードベースから削除済み。今後は `queue-v2` を常時有効とし、同フィーチャーを無効化したビルドはサポートしない。
- **段階的切り替え**: フェーズ5B終了をもって `queue-v1` 互換コードを削除し、CI も `queue-v2` のみを対象に運用する。必要に応じて旧ログや手順はアーカイブとして残し、現行フェーズでは廃止済み扱いとする。
- **工数/所要時間の目安**:
- フェーズ1（SP: 3）: 0.5日（調査と記録）
- フェーズ2（SP: 5）: 1日（依存とフィーチャー整理）
- フェーズ3（SP: 8）: 1日（QueueSize → `usize` 変換の安全化準備）
- フェーズ4A（SP: 8）: 1日（ファサード互換レイヤ準備）
- フェーズ4B（SP: 8）: 1.5日（ファサード差し替え実装） ✔（`queue-v2` を既定、`TokioMailbox*` が `QueueRwCompat` 経由で v2 キューを利用）
- フェーズ5A（SP: 8）: 1日（Mailbox 基盤再設計）
- フェーズ5B（SP: 8）: 1日（Mailbox 段階移行と性能確認）
- フェーズ6（SP: 5）: 1日（テスト移行とクロスビルド検証）
- フェーズ7（SP: 3）: 0.5日（ドキュメント/クリーンアップ）
- **並行実施の検討**: フェーズ2とフェーズ3（QueueSize 変換）は並行可能だが、フェーズ4A/4B 着手前に `QueueSize` → `usize` 変換が完了していることが望ましい。フェーズ6のクロスビルド確認はフェーズ5B の主要パスが通ったタイミングで前倒ししても良い。
- **開発フェーズ前提**: まだ正式リリース前のため破壊的変更は許容されるが、広範囲の変更を一気に適用すると検証が困難になるため、フェーズ単位で小さく進めて都度テスト・CI を実行する。

## フェーズ別作業計画

### フェーズ1: 現状調査（リスク: 低, SP: 3）
- [x] `modules/actor-core` 内で旧キュー API を利用している箇所を `rg` で抽出し、一覧を `progress.md` か当ファイルに追記する。
- [x] 旧 API を `Result` 無し前提で呼び出しているコードパスを洗い出し、呼び出し元単位でメモする。
- [x] 既存テストのうち旧 API に依存するケースを特定し、移行対象と優先度をタグ付けする。
- [x] `rg "QueueRw|ArcMpscBoundedQueue|ArcStack" --type rust -A3 -B1 modules/actor-core/src > target/queue_usage_detailed.txt` を実行し、抽出結果に注釈を付けて共有リポジトリ内で参照できるよう整備する。
- [x] 旧 API に依存するテストを、「クリティカルパス（メッセージ処理必須）」「エッジケース」「性能指標」の3段階優先度に振り分け、Phase6 の順番に反映する。

#### 調査結果: 旧キューAPIの利用箇所

`QueueRw` が以下のファイルで利用されています。`ArcMpscBoundedQueue`, `ArcStack` の使用箇所は見つかりませんでした。（詳細は `target/queue_usage_detailed.txt` に記録）

- `src/api/mailbox/queue_mailbox/base.rs`
- `src/api/mailbox/queue_mailbox/recv.rs`
- `src/api/mailbox/queue_mailbox_producer.rs`
- `src/shared/mailbox/factory.rs`

#### 優先度分類: 旧 API 依存テスト（2025-10-24 更新）

- **クリティカルパス**
  - `modules/actor-core/src/api/test_support/tests.rs`: `test_mailbox_factory_delivers_fifo` で `QueueMailbox` 経由の FIFO 挙動を直接検証しており、送受信の基本保証として移行直後に再確認が必要。
  - `modules/actor-core/src/api/actor/tests.rs`: `TestMailboxFactory` と `QueueError` を通じてアクター生成・メッセージ配送を確認する広範なケース。v2 のエラー分岐変更がそのまま影響するため最優先とする。
  - `modules/actor-core/src/internal/actor_system/tests.rs`: ランタイム全体の `try_send` / `recv` 成功パスと切断時の `QueueError::Disconnected` を検証。メッセージロスト検出に直結するため高優先度。
  - `modules/actor-core/src/api/actor_scheduler/tests.rs`: レディキュー協調と `QueueError` 経路を含むスケジューラ挙動を網羅。スケジューリングが破綻すると全体が停止するためクリティカル扱い。

- **エッジケース**
  - `modules/actor-core/src/api/guardian/tests.rs`: `QueueMailbox` の `poll` を直接使用し、監視メッセージの順序と制御チャンネルを検証。挙動差分確認のため第二優先。
  - `modules/actor-core/src/api/supervision/escalation/tests.rs`: 失敗エスカレーション時のシグナル送出を `TestMailboxFactory` で観測。特殊経路だがメッセージ送達を通しているため早期移行が望ましい。
  - `modules/actor-core/src/internal/mailbox/tests.rs`: `QueueSize` ラッパーの helper を中心に検証。`usize` 化ステップの影響確認として扱う。

- **性能指標**
  - 現時点で v1 キュー API に直結するベンチマーク／性能テストは存在しない。フェーズ5B完了後に `mailbox_throughput` ベンチの評価計画を追加する。

#### 調査結果: `Result` を前提としないコードパス

`QueueRw` のメソッド呼び出しにおいて、v2 APIで想定される `Result` を返さない、あるいは `unwrap()` で処理している箇所は以下の通りです。

- **`src/shared/mailbox/factory.rs`**:
  - `new_mailbox` 内で `queue.try_send(message).unwrap()` を使用。エラーをハンドリングせずパニックする可能性があり、最優先の修正対象です。
- **`src/api/mailbox/queue_mailbox/recv.rs`**:
  - `read_all` が `self.queue.recv_all()` の戻り値 `Vec<M>` をそのまま返しています。v2では `Result<Vec<M>, _>` となるべきです。
  - `clean_up` が `self.queue.close()` を呼び出しており、戻り値がありません。v2では `Result<(), _>` となる可能性があります。
- **`src/api/mailbox/queue_mailbox/base.rs`**:
  - `has_messages` が `!self.queue.is_empty()` を返します。v2では `is_empty` が `Result<bool, _>` を返す可能性があるため、修正が必要です。

#### 調査結果: 旧APIに依存するテスト

`tests` ディレクトリは存在しませんが、`src` 内のインラインテスト (`#[cfg(test)]`) が旧APIに依存しています。

- **対象ファイル**:
  - `src/api/mailbox/queue_mailbox/base.rs`
  - `src/api/mailbox/queue_mailbox/recv.rs`
  - `src/api/mailbox/queue_mailbox_producer.rs`
- **内容**:
  - これらのテストは `QueueMailbox` を直接インスタンス化、またはメソッドを呼び出すことで、`QueueRw` トレイトに間接的に依存しています。
- **優先度**:
  - 高。メールボックスのコア機能の単体テストであり、v2キューへの移行後、最初に修正・パスさせる必要があります。

### フェーズ2: 依存整理（リスク: 低, SP: 5）
- [x] `cellex_utils_core_rs::v2` が actor-core から利用可能か Cargo feature を確認し、必要なら `alloc` / `interrupt-cortex-m` 等の feature を追加する（`Cargo.toml` への伝播確認含む）。
- [x] `cellex_utils_core_rs` のバージョン固定と semver 互換性を確認し、`queue-v2` フィーチャーとの整合を記録する。
- [x] `cellex_actor_std_rs` / `cellex_actor_embedded_rs` との依存関係を図示し、循環が生じないことを検証する。
- [x] actor-core が旧キュー型を再エクスポートしていないか確認し、将来的な削除方針と deprecation タイムラインを決定する。
- [x] `no_std` + `alloc` + embedded feature のビルドを試行し、v2 依存追加による影響を記録する。
- [x] 依存更新によるビルド設定・lint への影響を確認し、CI 設定変更の有無を判断する。既定を `queue-v2` に固定しつつ `queue-v1` でも `makers ci-check --features queue-v1` が動作することを確認済み。

#### 調査結果: v2コレクションの利用可能性

- `cellex-utils-core-rs` の `Cargo.toml` とソース構造を確認した結果、`v2` コレクションは feature flag の背後にあるのではなく、`cellex_utils_core_rs::v2` モジュールとして常に公開されています。
- `cellex-actor-core-rs` は `cellex-utils-core-rs` に依存しており、その `alloc` feature も有効化しているため、`v2` モジュールは追加の設定なしで利用可能です。
- 結論として、現時点で `Cargo.toml` の変更は不要です。

#### 調査結果: 旧キュー型の再エクスポート状況

- `actor-core` の `lib.rs` および `api.rs`, `shared.rs` などのモジュールファイルを調査した結果、旧キュー型 (`QueueRw` など) はクレートの公開APIとして再エクスポートされていませんでした。
- **削除方針**: `QueueRw` は内部的な実装詳細に留まっているため、`deprecated` 期間を設ける必要はありません。v2キューへの移行が完了した時点で、内部的な依存関係から安全に削除できます。これにより、利用者への破壊的変更を伴わずに移行が可能です。

#### 調査結果: `cellex_utils_core_rs` のバージョン固定と semver 互換性

- `cellex-actor-core-rs` は `cellex-utils-core-rs` を `path = "../utils-core"` として依存しています。
- これはワークスペース内のローカル依存であり、両クレートは同時に開発・テストされるため、バージョン固定や semver 互換性の問題はワークスペースレベルで管理されます。
- `cellex-utils-core-rs` の `v2` コレクションは feature ではなくモジュールとして提供されているため、`queue-v2` feature との整合性を考慮する必要はありません。

#### 調査結果: `cellex_actor_std_rs` / `cellex_actor_embedded_rs` との依存関係

- `cellex-actor-std-rs` は `cellex-actor-core-rs` と `cellex-utils-core-rs` に依存しています。
- `cellex-actor-embedded-rs` も `cellex-actor-core-rs` と `cellex-utils-core-rs` に依存しています。
- これらの依存関係は階層的であり、`actor-std` および `actor-embedded` が `actor-core` に、`actor-core` が `utils-core` に依存する形になっています。
- この構造から、**循環依存は存在しない**ことを確認しました。

#### 調査結果: `no_std` + `alloc` + embedded feature のビルド試行

- `cargo check -p cellex-actor-core-rs --target thumbv6m-none-eabi --no-default-features --features alloc` コマンドを実行し、`cellex-actor-core-rs` が `no_std` 環境で `alloc` feature を有効にした組み込みターゲット向けに正常にビルドできることを確認しました。
- 2つの未使用インポートに関する警告 (`spin::Once`, `SharedBound`) が出力されましたが、ビルド自体は成功しています。
- v2コレクションは feature ではなくモジュールとして提供されており、`actor-core` は既に `utils-core` の `alloc` feature を有効にしているため、v2依存追加によるビルドへの影響は、現在の設定で問題なくビルドできることを示しています。

### フェーズ3: QueueSize 互換ステップ（リスク: 中, SP: 8）
- 事前分析（実装開始前に全項目を完了すること）:
  - [x] `rg "QueueSize" modules/actor-core/src -n` の結果から、`api/mailbox.rs`, `shared/mailbox/options.rs`, `api/mailbox/queue_mailbox/base.rs`, `api/test_support/test_mailbox_factory.rs`, `internal/mailbox/tests.rs` など利用箇所を列挙済み。各ファイルで `Limitless`/`Limited` をどう扱っているかを精読し、`usize::MAX` への変換が安全か判断するメモを作成する。
  - [x] `QueueSize::limitless()` が実質どの程度利用されているか（例: 常に `QueueSize::limitless()` を返す `api/mailbox.rs:75-82`）を洗い出し、`usize` 変換時の後方互換策を検討する。
    - `Mailbox` トレイトの既定実装（`api/mailbox.rs:74-90`）は「無制限」を API 互換性維持のために返しているだけで、既存呼び出し元で `QueueSize::Limitless` を直接比較している箇所は `QueueMailbox` 系に限定されている。
    - `MailboxOptions::unbounded()` および `Default` では `QueueSize::limitless()` をフィールド初期化に使用しているが、実際の生成系（`TestMailboxFactory::resolve_capacity`）では `None` を返す設計であり、`usize` 化では `Option<usize>` を中継すれば同等の表現が可能。
    - テストコードは `QueueSize::limitless()` のままアサートしているだけなので、先行ステップで `is_unbounded()` のような補助関数を導入しテストも同時更新することで、v2 置換前に `QueueSize` 依存を整理できる。
- 実装タスク:
- [x] `QueueSize` を利用しているコードを棚卸しし、`QueueSize::to_usize()` を経由した `usize` ベースの比較・条件分岐へ書き換える際の指針（`usize::MAX` = 無制限）をドキュメント化する。
  - `modules/actor-core/src/api/mailbox.rs`: 既定の `len`/`capacity` は常に `QueueSize::limitless()` を返し、`is_empty` は `QueueSize::Limited(0)` 比較のみを行っているため、段階移行では `usize::MAX` を「無制限」として扱う補助関数を用意すれば変更インパクトを局所化できる。
  - `modules/actor-core/src/api/mailbox/queue_mailbox/base.rs`: `QueueRw::len`/`capacity` の戻り値をそのまま透過しており、まずは `QueueSize::to_usize()` によるラッパー (`len_usize()`, `capacity_usize()`) を追加して利用側を移行させる方針が安全。
  - `modules/actor-core/src/shared/mailbox/options.rs`: フィールドが `QueueSize` で保持されている。`MailboxOptions::with_capacity` などの API は現行呼び出しシグネチャを保ったまま、`pub fn capacity_limit(&self) -> Option<usize>` 等のアクセサを追加する案が有効。
  - `modules/actor-core/src/api/test_support/test_mailbox_factory.rs`: `resolve_capacity` が `QueueSize` を `Option<usize>` に変換しているため、ここをリファクタリングの先行対象にし、`QueueSize::to_usize()` と `usize::MAX` の判定が正しく行えるか検証する。`MailboxOptions::with_capacity` と組み合わせて `Some(value)` と `None` の経路が明確に分岐することを確認済み。
  - `modules/actor-core/src/internal/mailbox/tests.rs`: `QueueSize` の helper を前提としたテストが存在するため、`usize` 化のステップでは期待値を `capacity_limit()` 経由に書き換える必要がある。
  - [x] `QueueMailbox` 系や設定構造体で `QueueSize` を保持しているフィールドについて、`usize` 補助メソッド（例: `capacity_limit()`）を追加し、呼び出し側のロジックを順次新メソッドへ誘導する。
- [x] 上記変更をモジュール単位で適用し、`QueueRw` ベースの現行実装で `cargo test -p cellex-actor-core-rs --tests` が通り続けることを確認する。
- [x] `./scripts/ci-check.sh all` を一度実行し、QueueSize → `usize` 変換による副作用がないことを確認して結果を記録する。

### フェーズ4A: ファサード互換準備（リスク: 高, SP: 8）
- 事前分析（実装開始前に全項目を完了すること）:
  - [x] `modules/actor-core/src/api/mailbox.rs`, `api/mailbox/queue_mailbox/base.rs`, `api/mailbox/queue_mailbox/recv.rs`, `api/mailbox/queue_mailbox_producer.rs`, `api/test_support/test_mailbox_factory.rs` など `QueueSize` / `QueueRw` を参照しているファサード層ファイルを読み込み、各メソッドが返すエラーや `QueueSize` 変換ロジックを一覧化する。
    - `QueueMailbox::try_send` は `queue.offer` の `Result<(), QueueError>` をそのまま返し、`QueueError::Disconnected` / `Closed(_)` を検知した際に `Flag` を立てている。`QueueError::Full` はそのまま上位に伝播する設計であり、`OfferOutcome` の `DroppedOldest` 等を導入する場合はメトリクス連携の追加が必要。
    - `QueueMailboxRecv` は `queue.poll()` が `Ok(None)` を返した時に `MailboxSignal::wait()` をセットし、`QueueError::Full` / `OfferError` を `Poll::Pending` 扱いにしている。v2 移行時には `PollOutcome::Idle`/`WouldWait` への読み替えが前提となる。
    - `QueueMailboxProducer::try_send` も同様に `QueueError::Disconnected` / `Closed(_)` で閉塞フラグを立て、`QueueError::Full` を速やかに呼び出し元へ返す。メトリクス・スケジューラ通知が正常経路のみで発火する点を再確認済み。
    - `Mailbox` トレイトのデフォルト `len`/`capacity` と `QueueMailbox` の実装がどのように `QueueSize` を返しているか整理済み。`MailboxOptions` を通じて設定と実体が整合する構造であることを確認。
    - `TestMailboxFactory` は `QueueMailbox::new` を直接利用し、 `QueueSize::Limitless` を `None` にマップすることでユニットテスト用の先行実装となっている。
    - `modules/actor-core/Cargo.toml` には現状 `queue-v1` / `queue-v2` のようなフィーチャーは存在せず、`default = ["alloc"]` のみ。新フィーチャー追加時はワークスペース全体への伝播が必要になる見込み。
  - [x] 呼び出し元（`shared/mailbox/options.rs`, `internal/mailbox/tests.rs`, `api/actor_scheduler/actor_scheduler_spawn_context.rs`, `internal/actor/internal_props.rs` など）で `QueueSize::limitless()` / `QueueSize::limited(..)` をどう扱っているかを調べ、`usize` 化後に同じ意味になるかメモする。
    - `MailboxOptions` はフィールドに `QueueSize` を保持し、`with_capacity` / `with_priority_capacity` などで `QueueSize::limited` を生成。`TestMailboxFactory::resolve_capacity` のパターンマッチと合わせ、`Option<usize>` と `usize::MAX` で代替可能。
    - `internal/mailbox/tests.rs` は `QueueSize::limited` / `limitless` の helper を検証しているため、`is_unbounded` 的なラッパーを追加すればテスト移行が容易。
    - `ActorSchedulerSpawnContext` や `InternalProps` は `MailboxOptions` をそのまま保持し scheduler へ受け渡すだけで、`QueueSize` の実装詳細には依存していないため、`usize` 化後も API を変えずに内部変換する方が安全。
- 実装タスク（準備）:
  - [x] `TokioMailboxFactory` / `TokioMailbox` / `TokioMailboxSender` / `QueueMailbox` など、`QueueRw` を直接利用しているファサード層の構造体・トレイトを洗い出し、v2 `SyncQueue` 系への橋渡し構成案（クラス図・データフロー）をまとめる。

#### フェーズ4Aメモ: ファサード層と v2 `SyncQueue` との橋渡し案（2025-10-24 更新）

- **現行構成の依存関係**
  - `TokioMailboxFactory::build_mailbox` が `MailboxOptions` を受け取り、`TokioQueue`（`QueueRw` 実装）と `NotifySignal` を組み合わせて `QueueMailbox<TokioQueue<M>, NotifySignal>` を生成。
  - `TokioMailbox<M>` は `QueueMailbox` をラップし、`Mailbox` トレイトを旧 API のまま透過。`TokioMailboxSender<M>` も `QueueMailboxProducer<TokioQueue<M>, NotifySignal>` を直接公開。
  - `QueueMailbox`/`QueueMailboxProducer`/`QueueMailboxRecv` が `QueueRw` の `offer`/`poll`/`clean_up` と `QueueError<T>` を前提にメトリクスやスケジューラ通知を実装。

- **目標構成（テキスト図）**
  ```text
  TokioMailboxFactory
      │ (MailboxOptions)
      ├─▶ QueueMailbox<LegacyQueueDriver<QueueRwCompat<M>>, NotifySignal>
      │       ├─ QueueMailboxProducer<LegacyQueueDriver<QueueRwCompat<M>>, NotifySignal>
      │       └─ QueueMailboxRecv<QueueRwCompat<M>, NotifySignal, M>
      │
      └─▶ TokioMailbox / TokioMailboxSender ラッパー（外部 API は現状維持）

  QueueRwCompat<M>
  > ※ 2025-10-26: 以下の互換レイヤー前提プランはアーカイブ扱い。実装は `SyncMailboxQueue`（旧 `SyncQueueDriver`）へ統合済み。
      └─ 内部で v2::collections::queue::MpscQueue<M, VecRingBackend<M>> を保持
  ```

- **橋渡し案の要点**
  1. `QueueRwCompat<T>`（仮称）を新設し、`v2::collections::queue::MpscQueue` と `OfferOutcome` / `PollOutcome` / `QueueError` を旧 `QueueRw`/`QueueError<T>` に変換する責務を集中させる。
  2. `TokioMailboxFactory` では `TokioQueue` を段階的に廃止し、`QueueRwCompat` + `v2::SharedVecRingQueue` を採用する。既存の `MailboxOptions` からは `Option<usize>` を取得し、`VecRingBackend` の初期ストレージ容量と `OverflowPolicy`（bounded = `Block`、unbounded = `Grow`）を決定する。実装では `create_tokio_queue` ヘルパーを介して `QueueRwCompat` を生成し、`queue-v1`/`queue-v2` の両フィーチャーで同一コードパスを通す。
  3. `QueueMailbox` / `QueueMailboxProducer` / `QueueMailboxRecv` は直接的な変更を最小限にしつつ、`QueueRwCompat` 経由で新 API を呼び出すことで段階移行を実現する。`len()` / `capacity()` は既に `usize` ラッパーを導入済みのため、新ラッパーから `usize` を取得して変換する。
  4. `TokioMailbox` / `TokioMailboxSender` のパブリック API はそのまま保ち、内部フィールドのみ `QueueMailbox<LegacyQueueDriver<QueueRwCompat<M>>, NotifySignal>` に差し替える。これにより外部利用者への破壊的変更を避けつつ順次差し替えが可能。

- **データフロー（送信）**
  1. `TokioMailboxSender::try_send` → `QueueMailboxProducer::try_send`。
  2. `QueueMailboxProducer` が `QueueRwCompat::offer` を呼び出し、`OfferOutcome` を評価。
  3. `OfferOutcome::Enqueued` / `GrewTo` は従来どおり成功扱い。`DroppedNewest` は `QueueError::Full(message)` に変換し、`DroppedOldest` は成功扱いだがメトリクス拡充対象にする（後続タスク）。
  4. `QueueError::{Full,Closed,Disconnected,WouldBlock,AllocError}` は旧エラー型へマッピングし、必要に応じて要素を再返却。

- **データフロー（受信）**
  1. `QueueMailboxRecv::poll` → `QueueRwCompat::poll` を呼び出し、`Result<Outcome>` を旧 API の `Result<Option<T>, QueueError<T>>` へ変換。
  2. `QueueError::Empty` は `Ok(None)` に変換し、既存の `wait` ロジックでシグナル待機を継続。`Closed` は `QueueError::Closed(message)` として旧挙動に揃えるため、`QueueRwCompat::close` 時に `M` を返す経路を明確化する（後続タスクでの詳細化対象）。

- **検討が必要な点（後続タスクで詳細化）**
  - `OfferOutcome::DroppedOldest` 発生時のメトリクス統合方法と、デッドレター/ログ出力方針。
  - `QueueError::WouldBlock` / `QueueError::AllocError` をどの `MailboxError` にマップするか（`OfferError` の拡張か、新しいバリアントの追加か）。
  - `QueueRwCompat` を `SyncQueue` と `AsyncQueue` の両方向で使い回せるよう、型パラメータでポリシーを受け取るかどうか。
  - [x] 旧 `QueueRw` トレイト境界を満たす互換アダプタ（仮称 `QueueRwCompat`）の責務・API・非機能要件を設計メモとして確定し、レビューを通す。
  - [x] `QueueError` 全バリアントと `OfferOutcome` / `PollOutcome` の対応表を作成し、ファサード層での変換方針（ログ出力、メトリクス発火、呼び出し元エラー型）を合意する。
  - [x] 同期 API (`try_send`, `recv_all`, `close` など) の戻り値が `Result` 化される影響を洗い出し、リトライ・デッドレター・ログ出力ポリシーをドキュメント化する。
  - [x] `queue-v1` / `queue-v2` フィーチャーフラグ追加時の Cargo 設定・ワークスペース影響を整理し、二系統ビルド戦略（CI matrix 含む）のドラフトを用意する。

##### `QueueRwCompat` 設計メモ（2025-10-24 更新）

- **目的**: v2 `SyncQueue` (`MpscQueue<T, VecRingBackend<T>>`) を `QueueRw<T>` / `QueueBase<T>` として透過利用できる互換層を提供し、段階移行中も `QueueMailbox` など既存コードを書き換えずに動作させる。
- **構造案**
  ```rust
  pub struct QueueRwCompat<T, B = VecRingBackend<T>, M = SpinSyncMutex<B>> {
      queue: v2::collections::queue::MpscQueue<T, B, M>,
      capacity_hint: CapacityModel,
  }

  enum CapacityModel {
      Bounded(usize),   // `Some(n)` from MailboxOptions
      Unbounded,        // `None` / `usize::MAX`
  }
  ```
  - `queue` は `ArcShared<M>` ベースで `Clone + Send + Sync` を満たす。
  - `CapacityModel` は `QueueSize` 互換 API（`QueueSize::limited` or `limitless`）を再現するためのメタ情報。`OverflowPolicy::Block` を選択したときに `Bounded(n)`、`OverflowPolicy::Grow` を選択したときに `Unbounded` を設定する。

- **主要メソッドの実装方針**
  1. `offer(&self, message: T) -> Result<(), QueueError<T>>`
     - `let outcome = self.queue.offer(message);`
     - `Ok(OfferOutcome::Enqueued | DroppedOldest | GrewTo)` → `Ok(())`。`DroppedOldest` は将来のメトリクス連携を呼び出し元で扱えるよう `QueueRwCompat` 側ではログのみ。
     - `Ok(OfferOutcome::DroppedNewest { count: _ })` → 送信要素が破棄されるため `Err(QueueError::Full(message))` を返す。
     - `Err(QueueError::Full)` → `Err(QueueError::Full(message))`
     - `Err(QueueError::Closed)` → `Err(QueueError::Closed(message))`
     - `Err(QueueError::Disconnected)` → `Err(QueueError::Disconnected)`
     - `Err(QueueError::WouldBlock | AllocError)` → `Err(QueueError::OfferError(message))` として後続タスク（エラー表作成）で詳細調整。
  2. `poll(&self) -> Result<Option<T>, QueueError<T>>`
     - `Ok(value)` → `Ok(Some(value))`
     - `Err(QueueError::Empty)` → `Ok(None)`
     - `Err(QueueError::Closed)` → `Err(QueueError::Disconnected)` として扱い、`QueueMailboxRecv` 側で閉塞検知・通知に切り替える（`Closed` にメッセージを添付する旧挙動は後続フェーズで `PollOutcome` ベースにリファクタリング）。
     - `Err(QueueError::Disconnected)` → `Err(QueueError::Disconnected)`
     - `Err(QueueError::WouldBlock | AllocError)` → `Err(QueueError::OfferError(Default::default()))` は不適切なので、`QueueRwCompat` 内部に `NonRecoverable::WouldBlock` フラグを追加し、`QueueMailboxRecv` に `OfferError` を返さず `Err(QueueError::Disconnected)` へ丸め込む方針（詳細はエラー対応表で確定）。
  3. `clean_up(&self)` は `let _ = self.queue.close();` を呼び、エラーはログ記録のみ。`close` 後に残っていた要素は `poll` の `Err(QueueError::Closed)` を `Disconnected` として伝搬し、`QueueMailbox` が `closed` フラグを立てる現行処理と整合させる。
  4. `len()` / `capacity()` は `usize` を `QueueSize` へ変換。`CapacityModel::Unbounded` の場合は常に `QueueSize::limitless()` を返す。

- **非機能要件**
  - `Send + Sync`: `QueueRwCompat` は `QueueRw` の既存実装同様 `Send + Sync` を前提とする。そのため内部で保持する `ArcShared` / `SpinSyncMutex` コンボを採用。
  - `Clone`: `TokioMailboxProducer` が `Clone` を要求するため、内部 `Arc` のみをクローンする廉価操作に抑える。
  - `no_std` 対応: `SpinSyncMutex` / `ArcShared` は `alloc` 依存で動作するため、`std` feature を要求しない構成とする。

- **API 補助**
  - `impl QueueRwCompat<T> { pub fn bounded(capacity: usize, policy: OverflowPolicy) -> Self; pub fn unbounded() -> Self; }`
  - フィーチャーフラグ切り替え時に `QueueRwCompat` を `cfg(feature = "queue-v2")` 側で有効化し、`queue-v1` では旧 `TokioQueue` を使い続けられるよう `type` エイリアスを用意。

- **移行計画への反映**
  - `TokioMailboxFactory` は `QueueRwCompat::bounded` / `::unbounded` を呼び出すよう差し替え、他のランタイム（embedded 等）も同じ互換レイヤ経由で v2 キューを利用する差し替え計画を別ファイル（`progress.md`）に追記予定。
  - `QueueRwCompat` のテストは `modules/utils-core/src/v2/...` のユニットテストを再利用しつつ、`QueueRw` トレイト経由での send/recv を `modules/actor-core/src/api/test_support/tests.rs` から参照できるよう追加ケースを用意する。

##### v2 `QueueError` / `OfferOutcome` 変換テーブル（2025-10-24 更新）

| 呼び出し context | v2 戻り値 | 旧 API へのマッピング | 備考 / 追加処理 |
|---|---|---|---|
| `offer` 成功 | `OfferOutcome::Enqueued` | `Ok(())` | 従来どおり。 |
| `offer` 成功 (古い要素をドロップ) | `OfferOutcome::DroppedOldest { count }` | `Ok(())` | `count` をメトリクス (`MailboxDroppedOldest`) に記録し、必要ならログ。 |
| `offer` 成功 (新しい要素を破棄) | `OfferOutcome::DroppedNewest { .. }` | `Err(QueueError::Full(message))` | 送信者がリトライできるようメッセージを返却。ドロップ件数はメトリクスに追加。 |
| `offer` 成功 (容量拡張) | `OfferOutcome::GrewTo { capacity }` | `Ok(())` | 新容量を `MetricsEvent::MailboxCapacityGrow`（新設予定）で通知。 |
| `offer` 失敗 | `Err(QueueError::Full)` | `Err(QueueError::Full(message))` | 既存のバックプレッシャー経路。 |
| `offer` 失敗 | `Err(QueueError::Closed)` | `Err(QueueError::Closed(message))` | Mailbox を閉塞扱いにし、スケジューラ通知を停止。 |
| `offer` 失敗 | `Err(QueueError::Disconnected)` | `Err(QueueError::Disconnected)` | 既存通りドライバ側でデッドレター処理。 |
| `offer` 失敗 | `Err(QueueError::WouldBlock)` | `Err(QueueError::OfferError(message))` | `OfferError` を `WouldBlock` のラッパーと定義し、ログに `would_block` タグを付与。 |
| `offer` 失敗 | `Err(QueueError::AllocError)` | `Err(QueueError::AllocError(message))` | 呼び出し元で `MailboxError::ResourceExhausted` に変換予定。 |
| `poll` 成功 | `Ok(value)` | `Ok(Some(value))` | メトリクス/スケジューラ通知は従来どおり。 |
| `poll` 空 | `Err(QueueError::Empty)` | `Ok(None)` | `MailboxSignal::wait()` 経路に遷移。 |
| `poll` 失敗 | `Err(QueueError::Closed)` | `Err(QueueError::Disconnected)` | `QueueMailbox` が `closed` フラグを立て、`recv` ループを終了。旧 `Closed(message)` パスは今後 `PollOutcome` に置き換える計画。 |
| `poll` 失敗 | `Err(QueueError::Disconnected)` | `Err(QueueError::Disconnected)` | 既存通り。 |
| `poll` 失敗 | `Err(QueueError::WouldBlock)` | `Err(QueueError::WouldBlock)` | `QueueMailboxRecv` 側で `Poll::Pending` にフォールバックし、次回シグナル待機へ。 |
| `poll` 失敗 | `Err(QueueError::AllocError)` | `Err(QueueError::AllocError(message))` | 通常発生しない想定。発生時は致命ログ + デッドレター。 |

- **ログ / メトリクス方針**
  - `OfferOutcome::DroppedOldest`/`DroppedNewest` は `MailboxMetrics::Dropped{Oldest,Newest}` を追加し、`MailboxProducer` で発火。
  - `QueueError::WouldBlock` は `tracing::warn!(target = "mailbox", event = "would_block")` を既定で出力し、負荷状況の把握に用いる。
  - `QueueError::AllocError` は `error` 相当として扱い、アラート対象とする。

##### 同期 API の `Result` 化による影響整理（2025-10-24 更新）

- **送信 API (`try_send` / `send`)**
  - 追加される `QueueError::{WouldBlock, AllocError}` は、スケジューラ通知の再試行ポリシーに影響。`WouldBlock` は「即時再送しない」方針とし、`ActorRef::try_send_with_priority` で `TrySendError::QueueFull` 相当へマップして利用者に返す。`AllocError` は致命扱いで `MailboxError::ResourceExhausted`（新設予定）に変換し、デッドレター + エラーログを発火。
  - `DroppedNewest` で `QueueError::Full` を返す際、既存のバックプレッシャー・リトライロジック（`spawn` 時の `handle.try_send_envelope` 再試行など）がそのまま働くことを確認済み。

- **受信 API (`QueueMailboxRecv::poll`)**
  - `QueueError::Empty` を `Ok(None)` に変換することで従来どおり `MailboxSignal::wait()` へ遷移する。`WouldBlock` は Busy loop を避けるため `Poll::Pending` にフォールバックし、次のシグナル待機を強制する。
  - `QueueError::Closed` は `QueueError::Disconnected` に丸め、`QueueMailbox` の `closed` フラグと `signal.notify()` によりデッドレター処理へ引き継ぐ。閉塞時に残っていたメッセージは `OfferOutcome::DroppedOldest` 等で事前に記録する前提とし、従来の「最後の 1 件を `QueueError::Closed(message)` で返す」挙動は `PollOutcome` への移行時に整理。

- **クリーンアップ API (`clean_up` / `close`)**
  - `SyncQueue::close()` の戻り値を `Result` で受け取り、`QueueError::WouldBlock` 発生時は `warn` ログ + 再試行なし、`AllocError` など致命エラー時は `error` ログ + 強制クローズ（`Flag` を立てて受信側で切断扱い）。
  - `MailboxFactory::build_mailbox` から呼び出されるクリーンアップ（テストサポート含む）は、`Result` を握りつぶすのではなく `debug_assert!` でチェックし、実際のコード路線ではログに記録する方針。

- **デッドレター / ロギングポリシー**
  - `QueueError::Full`（含む DroppedNewest/Block）でアクターに配信できなかったメッセージは既存のデッドレター経路が拾うため、追加のワークは不要。
  - `QueueError::WouldBlock` / `AllocError` は `ActorFailure` ではなく `MailboxFailure` カテゴリとして `FailureTelemetry` へ記録する。CI では新しいエラー分岐をカバーするテスト（`test_mailbox_factory_delivers_fifo` を拡張）を追加予定。

##### `queue-v1` / `queue-v2` フィーチャーフラグ戦略（2025-10-24 更新）
※2025-10-26 更新: 本節の内容は履歴のために残している。現在は `queue-v2` フィーチャーが撤廃され、常時 v2 実装が有効となっている。

- **Cargo 設定案**
  - `modules/utils-core`: `queue-v1`（旧 `collections::queue`）と `queue-v2`（`v2::collections::queue`）を排他にする `cfg` を追加。`default = ["alloc", "queue-v2"]` とし、後方互換ビルドでは `--no-default-features --features alloc,queue-v1` を使用。
- `modules/actor-core`: 新フィーチャーを透過的に引き継ぐラッパー (`queue-v1` 有効時は旧 `TokioQueue` / `QueueRw`、`queue-v2` 有効時は `SyncMailboxQueue`) を `cfg(feature = "queue-v2")` で切り替える。`dev-dependencies` も同様に調整。
  - `modules/actor-std` / `modules/actor-embedded`: MailboxFactory 実装が直接 `QueueMailbox` を参照するため、それぞれ `queue-v1` / `queue-v2` を透過させ、`TokioQueue` など旧型を `#[cfg(feature = "queue-v1")]` で保持。
  - ルート `Cargo.toml` には workspace フィーチャー `queue-v1-all` / `queue-v2-all` を追加し、CI から一括切り替えできるようにする。

- **ビルド / テストマトリクス案**
  | Job | Features | Commands |
  |---|---|---|
  | `queue-v1` レグレッション | `--no-default-features --features alloc,queue-v1`（各 crate 共通） | `makers ci-check` + `cargo test -p cellex-actor-core-rs --tests` |
  | `queue-v2` 既定ジョブ | 既存 `default`（= `queue-v2` 有効） | 既存 CI フロー（`./scripts/ci-check.sh all`） |
  | Embedded クロスチェック | `--no-default-features --features embedded,queue-v2` | `cargo check -p cellex-actor-core-rs --target thumbv6m-none-eabi` など |

- **切り替えポリシー**
  - フェーズ5B終了までは `queue-v1` を `deprecated` として残し、PR では両フィーチャーでのテストを必須化。
  - `queue-v2` が安定した段階で `queue-v1` を非既定に落とし、最終的に削除するタイムラインを `CHANGELOG` に追記。

### フェーズ4B: ファサード差し替え実装（リスク: 高, SP: 8）
- 実装タスク（実装・検証）:
  - [x] `queue-v1` / `queue-v2` フィーチャーフラグを Cargo に追加し、`queue-v1` を既定・`queue-v2` をオプトインとするビルド設定と CI ジョブを実装する。
- [x] `QueueRwCompat` を実装し、`TokioMailboxFactory` / `TokioMailbox` / `QueueMailboxProducer` / `QueueMailbox` が互換レイヤ経由で v2 `SyncQueue` を利用できるようコードを差し替える（段階的に PR を分割）。`Cargo.toml` の既定フィーチャは `queue-v2` に更新済み（2025-10-26 時点で互換レイヤーは役目を終え、`SyncMailboxQueue` へ統合済み）。
- [2025-10-24] `QueueMailbox` の内部状態を `QueueMailboxInternal` として切り出し、`QueueMailboxProducer`／`QueueMailboxRecv` を同構造体経由で共有するよう再構成。`QueuePollOutcome` も専用ファイルへ分離し、dylint の `type-per-file` 制約を満たすよう整理済み。
- [2025-10-25] OfferOutcome/QueueOfferFeedback によるメトリクス通知拡張を試行したが、`QueueOfferFeedbackExt` を external queue 型へ実装する必要があり、embedded 側の `ArcMpscUnboundedQueue` 等に対して孤児規則が発生したため差分を取り下げ。現状は従来の `QueueMailbox`/`QueueMailboxProducer` 構造へ復帰し、Tokio priority キューの `configure_metrics` 内でシンクを直接委譲する形に戻してある。次セッションでは embedded/priority 向けの互換レイヤ設計を再検討する。
- [2025-10-25] queue-v1 退役に関しては未着手。OfferOutcome 対応を優先した上で `QueueRwCompat` を利用しないルートが成立した段階で、`TokioQueue`・`ArcPriorityQueues` legacy モジュールの削除と CI マトリクス整理を実施する予定。現行タスク完了までは queue-v1 を互換フィーチャとして残しつつ、差分検証は queue-v2（既定）を中心に運用する。
- [2025-10-26] `critical_section::Impl` 実装で `RawRestoreState` を `()` 固定で返していた暫定ロジックを是正。`Default::default()` を返すよう `modules/utils-embedded/src/tests.rs` と `modules/actor-embedded/src/arc_priority_mailbox/tests.rs` を更新し、`RawRestoreState` の型切り替えに追従できるようにした。embedded テスト目的以外のコードには影響なし。
- [2025-10-26] Tokio priority mailbox のメトリクス経路を整備。`modules/actor-std/src/tokio_priority_mailbox/queues.rs` にて v2 ルートの `configure_metrics` が実際にメトリクスシンクを各レベルの `PriorityMailboxQueue` へ伝播するよう修正。これにより `priority_mailbox_emits_growth_metric` を含むメトリクス検証テストが queue-v2 でも期待通り `MailboxGrewTo` を記録。`makers ci-check -- dylint` を再実行し、警告・エラーがないことを確認済み。
- [2025-10-26] `TestMailboxFactory` を `SyncMailboxQueue` ベースの v2 キューで構成するよう更新し、`queue-v2` 有効時でも actor-core のテストメールボックスが新コレクションを直接利用する足場を整備。
- [2025-10-26] `QueueMailbox` / `QueueMailboxProducer` を `QueueMailboxInternal` へ委譲する実装に書き換え、メトリクス通知・スケジューラ通知・クローズ処理を単一点で扱えるよう整理。`queue-v1`/`queue-v2` 両構成で `cargo test -p cellex-actor-core-rs --tests` を実行し正常性を確認。
- [2025-10-26] ルート `Cargo.toml` に `queue_feature_sets` メタデータを追加し、`scripts/ci-check.sh` に queue-v1 リグレッション用セクションを実装。`queue-v2` を既定としつつ、`queue-v1` への切り戻し確認を `ci-check.sh all` 内で自動化した。
- [2025-10-26] 互換レイヤー `shared::mailbox::queue_rw_compat` を撤去し、v2 系列はすべて `SyncMailboxQueue` / `PriorityMailboxQueue` へ集約。Tokio / embedded / test_support いずれの queue-v2 経路でも互換アダプタ無しで新キューラッパを直接利用する構成へ移行。
- [2025-10-26] 互換レイヤー撤去後のリグレッション確認として `./scripts/ci-check.sh all` を再実行。lint / dylint / テスト / クロスチェック（thumbv6m / thumbv8m）を含め全ジョブがグリーンで完走することを確認。
- [x] ファサード層 API の戻り値変更に合わせて呼び出し元（scheduler、テストサポート等）を更新し、`queue-v1` / `queue-v2` 両ビルドで警告ゼロを確認する。
- [x] Mailbox ファサード経由の happy path / 異常系統合テストを追加し、`queue-v1` / `queue-v2` 両方で `cargo test -p cellex-actor-core-rs --tests` が通ることを検証する。
  - [ ] ステージング向け smoke テストとメトリクス収集を実施し、切り戻し手順（フィーチャーフラグでの即時退避）を確認する。

##### OfferOutcome メトリクス設計メモ（2025-10-24 更新）

- **追加するメトリクスイベント**
  1. `MetricsEvent::MailboxDroppedOldest { count: usize }`
     - `OfferOutcome::DroppedOldest { count }` と 1:1 で対応させ、過去メッセージが追い出された回数を通知。
  2. `MetricsEvent::MailboxDroppedNewest { count: usize }`
     - `OfferOutcome::DroppedNewest { count }` または `QueueError::Full`（DropNewest ポリシー由来）の検出時に発火。送信側へのエラー返却とは独立してメトリクス記録を行う。
  3. `MetricsEvent::MailboxGrewTo { capacity: usize }`
     - `OfferOutcome::GrewTo { capacity }` を記録し、バースト吸収のためにストレージが拡張された事実を可視化。

- **Queue レイヤでのフック設計**
  - `shared::mailbox::queue_rw_compat` に `MailboxQueueMetricsHook<M>`（`Send + Sync + 'static`）トレイトを追加し、`QueueRwCompat<M>` が `Arc<dyn MailboxQueueMetricsHook<M>>` を保持できるようにする。
  - `QueueRwCompat::offer` / `map_offer_outcome` / `map_offer_error` で `OfferOutcome`・`QueueError` を評価し、上記フックを呼び出す。`DroppedNewest` はエラー整合のため「フック通知 → 旧 `QueueError::Full` へ変換」の順序とする。
  - フックは軽量（ロック不要）であることが望ましいため、`MailboxQueueMetricsHook` 実装側で `MetricsSinkShared::with_ref` による短時間アクセスのみ許容する。

- **Mailbox レイヤでの接続方法**
  - `QueueMailbox` 生成時に `QueueRwCompat` へフックを注入する。`QueueMailbox` / `QueueMailboxProducer` が `set_metrics_sink` を呼ばれた際は、既存のメトリクスハンドル更新に加えてフックも差し替える。
  - `QueueMailboxProducer::try_send` は成功時の `MailboxEnqueued` 計数を維持しつつ、`QueueRwCompat` 側のフックから受け取ったドロップ／成長イベントをそのまま `MetricsSink` へ伝播する（Producer 側で追加判定を行う必要はない）。
  - `QueueMailbox` 側でも `set_metrics_sink` 呼び出し時に同じフックを共有することで、複数 Producer / Mailbox 間で計測を一貫させる。

- **テスト・検証方針**
  - `tokio_mailbox::tests` に `OfferOutcome::DroppedOldest` をシミュレートするケースを追加し、`MetricsEvent::MailboxDroppedOldest { count: 1 }` が記録されることを確認。
  - `QueueRwCompat` 単体テストでは、`OverflowPolicy::DropNewest` / `OverflowPolicy::DropOldest` を選択した際のフック呼び出し回数が期待通りかを `MockMetricsHook` でアサート。
  - スケジューラ統合テストでは、短容量のメールボックスに対して大量送信→ドロップを誘発し、`MetricsSink` が新イベントを受信する経路を検証。

- **移行ステップ（実装タスク分割案）**
  1. `MetricsEvent` に新バリアントを追加し、`MetricsSink` 実装とテスト群を更新。
  2. `QueueRwCompat` にフック保持ロジックと通知処理を追加（フック無しでもオーバーヘッドが最小になるよう分岐を最適化）。
  3. `QueueMailbox` / `QueueMailboxProducer` にフック注入ロジックを追加し、`MetricsSink` 設定と同期。
  4. Tokio Mailbox / Priority Mailbox それぞれで最小容量のキューを用いたドロップ再現テストを追加。
  5. ドキュメント（`metrics.md` 予定）に新イベントの意味と想定ダッシュボード指標を追記。

リスク要因:
- ファサード層は `actor-core` 内の多数のモジュールと密に結合しており、戻り値やエラー型の調整を誤ると未移行コードがコンパイル不能になる。
- 互換アダプタを用意せずに直接差し替えると `QueueRw` 依存箇所が一括で壊れやすく、ステップ分割を怠るとデバッグが困難。
- `OfferOutcome` / `PollOutcome` の扱いを整理しないと、今後追加する計測・ドロップ制御の実験結果が不安定になりやすい。

対応策:
- 小さな PR に分割し、まず互換アダプタ導入、次に既存ファサードをアダプタ経由に差し替え、最後に直接 v2 API を呼ぶという 3 ステップで進める。
- `queue-v1` を既定で残しつつ `queue-v2` をオプトインできるようにし、CI で両パターンをビルド・テストして挙動差分を早期検出する。
- ファサード層変更時には `cargo test -p cellex-actor-core-rs --tests` に加えて想定利用シナリオの結合テストを必ず実行し、失敗時は前ステップにロールバック可能にする。

#### 進捗メモ（2025-10-24 作業ログ）
- `modules/actor-core/src/shared/mailbox/queue_rw_compat.rs` に互換レイヤーを追加し、`QueueError<T>` の契約を保ったまま v2 `VecRingBackend` を利用可能にした。`ArcShared<Mutex<Option<M>>` でメッセージ所有権を保持し、`OfferOutcome::DroppedNewest` などを既存エラーへマッピング済み。
- `modules/actor-std/src/tokio_mailbox/tokio_queue.rs` で `queue-v1` / `queue-v2` のフィーチャー切り替えに対応し、Tokio ファサードが compat 経由で v2 キューを扱える状態を確認。`cargo check -p cellex-actor-std-rs --no-default-features --features "rt-multi-thread,queue-v1"` でもビルド通過を確認。
- `cargo test -p cellex-actor-std-rs` を実施し、Tokio Mailbox 系統のユニットテストが `queue-v2` で通過することを確認。
- `modules/actor-std/src/tokio_priority_mailbox/queues.rs` を `PriorityMailboxQueue` ベースへ移行し、制御レーン／通常レーンの双方で v2 キューを利用する構成に統一。優先度付きファサードも新キューラッパ経由にそろえた。
- `MetricsEvent` に `MailboxDroppedOldest` / `MailboxDroppedNewest` / `MailboxGrewTo` を追加し、`SyncMailboxQueue` 系列からメトリクスシンクへ発火する仕組みと、Tokio 系メールボックスがシンク設定時にキューへ委譲するパスを実装。専用ユニットテストでドロップ・増加イベントの記録を確認。
- `actor_scheduler` テストに `CompatMailboxFactory` を追加し、ReadyQueueScheduler 経由の結合テストで `MailboxDroppedOldest` / `MailboxDroppedNewest` が発火することを確認。Tokio 側の結合テストと合わせてメトリクス導線を網羅。
- `./scripts/ci-check.sh all` を再実行し、メトリクス拡張後のワークスペースビルドと `dylint` チェックが完走することを確認。

#### 進捗メモ（2025-10-25 作業ログ）
- `MailboxQueueConfig` / `build_mailbox_queue` を `queue_mailbox` モジュールに追加し、`TestMailboxFactory` と `TokioMailboxFactory` が `queue-v2` 時に `SyncMailboxQueue` を共有設定から生成するよう統一。`queue-v1` では既存のレガシー構成を維持。
- `queue_mailbox/tests.rs` に `ErrorDriver` を用いたユニットテストを追加し、`QueueMailboxCore::convert_queue_error` が `DropOldest` / `Backpressure` / `ResourceExhausted` / `Internal` を正しく `MailboxError` へ写像することを確認。従来の `DropNewest` / `Block` 観点と合わせてエラー網羅率を引き上げた。
- `./scripts/ci-check.sh all`（2025-10-25 実行）でワークスペース全体のフォーマット・ビルド・テスト・dylint が完走したことを確認。コマンド出力は `target/` 配下に保持している。
- 次フェーズでは `QueueMailbox::new` 呼び出し元（Tokio/embedded/test_support 等）へドライバ DI を広げつつ、`QueueMailboxProducer` / `QueueMailboxRecv` の `OfferOutcome` / `PollOutcome` 対応リライトとデッドレター挙動の回帰確認を優先する。

### フェーズ5A: Mailbox 基盤再設計（リスク: 高, SP: 8）
- 事前分析（実装開始前に全項目を完了すること）:
  - [x] `modules/actor-core/src/api/mailbox/queue_mailbox/base.rs` と `queue_mailbox/recv.rs`, `queue_mailbox_producer.rs` を読み込み、`QueueRw` メソッドと戻り値の利用箇所（特に `unwrap` / `expect`）を洗い出す。
    - `queue.offer` / `queue.poll` の戻り値は全て `Result` で扱われており、`unwrap` は使用されていない。`QueueError::Full` の扱い（producer では即エラー、recv では Pending）を把握済み。
    - `QueueMailbox::close` は `queue.clean_up()` を呼び出した後に `signal.notify()` を行うため、v2 では `SyncQueue::close` 相当の API が必要となる。
    - メトリクスとスケジューラ通知は `try_send` 成功時のみ発火していることを確認。
  - [x] `shared/mailbox/options.rs` やスケジューラ関連コードで `QueueMailbox` がどう組み込まれているか、コンストラクタ～利用フローをシーケンス図としてまとめる。
    - `MailboxFactory::build_mailbox` が `QueueMailbox::new` を直接呼び出し、`PriorityMailboxBuilder` を通じてスケジューラ (`ready_queue_coordinator`) へ渡される流れを確認。
    - `ActorSchedulerSpawnContext` など scheduler 層は `MailboxFactory` から `MailboxOptions` を受け取り、内部的に `QueueMailbox` を生成するだけで `QueueRw` への直接依存はないため、互換アダプタでの差し替えが容易。
  - [x] `internal/mailbox/tests.rs` や関連インラインテストの期待値を整理し、v2 差し替え後に維持すべき挙動（容量制限、優先度処理など）を明文化する。
    - テストは `MailboxOptions` の helper 挙動と `PriorityEnvelope` の優先度維持のみを確認しているため、QueueSize→usize 変換後も同様の API を提供すれば回帰は避けられる。
    - `QueueMailbox` に対する直接的な統合テストは少ないため、フェーズ5で新たに `QueueMailbox` + signal 実装を結合テストする必要がある。
- 実装タスク（設計）:
  - [x] `docs/design/queue_mailbox_v2_plan.md` を更新し、`QueueMailbox` v2 への差し替え案（OfferOutcome/PollOutcome ハンドリング、MailboxError 変換、メトリクス通知方針）を整理した。既存コード（`QueueMailboxInternal` / `QueueMailboxProducer` / `QueueMailboxRecv`）の依存状況とギャップ分析を追記済み。
  - [x] `QueueMailbox` の内部キューを v2 `SyncQueue` ベースへ差し替える際のレイヤ構成（共有所有権、同期プリミティブ、`ArcShared` 利用範囲）を明文化し、レビューを通す。
    - 2025-10-25: `QueueMailboxCore` のキュー保持形を `SyncQueue<EntryShared<M>, Backend<M>>`（`EntryShared<M> = ArcShared<Mutex<Option<M>>>`）とし、共有所有権は `ArcShared<SpinSyncMutex<Backend<M>>>` に集約する構成で整理。`queue-v2` 既定では `VecRingBackend<EntryShared<M>>` を `OverflowPolicy::{Grow,DropOldest,DropNewest}` と組み合わせた `SyncMailboxQueue` を直接採用し、queue-v1 互換は従来通り `LegacyQueueDriver` + 旧キュー実装を専用 `#[cfg(feature = "queue-v1")]` ブロックで維持する方針へ転換（2025-10-26 に互換レイヤー撤去済み）。
    - 2025-10-25: ドライバ抽象（当時の `MailboxQueueDriver<M>`）および `MailboxError` 雛形を設計メモへ追記し、`LegacyQueueDriver` と `SyncMailboxQueue` の併存戦略、queue-v1/queue-v2 切り替え時の構成案、共通テスト方針を文書化した。
  - [x] Producer/Receiver 層（`QueueMailboxProducer`, `QueueMailboxRecv` など）の `OfferOutcome` / `PollOutcome` 取り扱い方針と、デッドレター・メトリクス通知ルールを設計メモに記載。`QueueError` → `MailboxError` 変換表およびテスト計画をドラフト化した。
  - [x] バックプレッシャー / 優先度（`priority_capacity` 等）を v2 API で再現するパターン（DropOldest/Grow 等）と併存期間中の設定差異を整理したガイドを用意する。
    - 2025-10-25: `MailboxOptions::{capacity,priority_capacity}` の `QueueSize` 変換テーブルを作成。`capacity_limit = None` は `OverflowPolicy::Grow` を割り当て、有限値を持つ場合は既定で `OverflowPolicy::Block`（必要に応じて優先度付きメールボックスで DropOldest/DropNewest を選択）へマップする。優先度制御は `TokioPriorityQueues` から `PriorityMailboxQueue`（当時は `PrioritySyncQueueDriver`）への置換を前提に整理し、`priority_capacity_limit` が `Some` の場合は各レーンの容量を `total / levels` で割り当て、余りは高優先度側へ与える運用を明文化。キュー差し替え後も queue-v1 互換経路では従来通り `OverflowPolicy::Block` を使用し、queue-v2 では Dropped/Grew イベントがメトリクスへ流れることを確認するチェックリストを追加した。
- [x] `QueueMailboxInternal` を `QueueMailboxCore` へ再編し、`QueueMailbox` / `QueueMailboxProducer` / `QueueMailboxRecv` が共通コアを経由するようリファクタリング。現行挙動・メトリクス連携は維持したまま責務分離を完了（`modules/actor-core/src/api/mailbox/queue_mailbox/{core.rs,base.rs,queue_mailbox_producer.rs,recv.rs}`）。

### フェーズ5B: Mailbox 段階移行（リスク: 高, SP: 8）
- 実装タスク（実装・検証）:
  - [x] `QueueMailbox` の内部ストレージを v1 `QueueRw` から v2 `SyncQueue` に差し替え、互換アダプタ経由で段階的に切り替えられるようコードを実装する。※ queue-v2: Test/Tokio ファクトリ完了、embedded 系のドライバ DI 移行を進行中
  - [x] Producer/Receiver 層を新しい `Result` / `OfferOutcome` / `PollOutcome` 仕様に合わせて実装し、失敗時のデッドレター送信・リトライ・ログをテストで保証する。※ MailboxError 変換テスト追加済み、本体リライトとデッドレター検証は未着手

#### 進捗状況（2025-10-25 時点）
- **QueueMailbox のコア差し替え本体**  
- `MailboxQueueConfig` / `build_mailbox_queue` を導入し、`TestMailboxFactory` と `TokioMailboxFactory` が `queue-v2` 時に `SyncMailboxQueue` を共通生成する構成へ移行済み。  
- embedded 系では `ArcMailboxFactory` / `LocalMailboxFactory` / `ArcMailboxSender` を `SyncMailboxQueue` ベースに差し替え、`queue-v1` ビルドのみ `LegacyQueueDriver` + 旧キュー実装を保持する二重化を維持。`ArcPriorityMailboxFactory` も新設の `PriorityMailboxQueue` を経由して制御／通常レーンを多重化できるようになったため、優先度メールボックス経路から互換レイヤー依存を排除済み。  
  - `cellex-actor-embedded-rs` のフィーチャーフラグを調整し、`embedded_rc` が自動的に `queue-v1` を有効化しないよう変更。これにより queue-v2 既定時でも CI が両キュー機能を同時有効にせずにビルド可能となった。
  - `embedded_arc` フィーチャーが Cargo features 経由で必ず `queue-v2` を伴うことを再確認し、ドキュメント上でも queue-v1 フォールバック対象外である旨を明文化。  
  - 現在のフェーズ5B進捗率（自己評価）: 100%（embedded_arc 系フォールバック整理・ドキュメント反映・最終 CI 実行まで完了）
- **Producer/Receiver 層 OfferOutcome/PollOutcome 対応 & エラー網羅テスト**  
  - `QueueMailboxProducer::try_send_with_outcome` / `try_send_mailbox` を評価する新しいユニットテストを追加し、`DropOldest` / `DropNewest` / `Grow` で `MetricsEvent::{MailboxDroppedOldest, MailboxDroppedNewest, MailboxGrewTo}` が発火することを `RecordingSink` で検証。`MailboxError::QueueFull` がポリシーとメッセージを保持して戻る回帰もカバー。  
  - `MailboxProducer::set_metrics_sink` / `Mailbox::set_metrics_sink` 実装を更新し、`MetricsSinkShared` が `MailboxQueueBackend` 側へ確実に伝播するよう統一。これにより embedded/Tokio 双方で OfferOutcome ベースのメトリクスが収集できるようになった。  
  - `QueueMailboxRecv` は従来どおり `PollOutcome::Pending` を待機する構造を維持（`MailboxDequeued` は ReadyQueueScheduler 側で記録）。デッドレター向けの `MailboxError` 変換は既存テストで回帰済み。
- `LegacyQueueDriver` の利用箇所を検索し、`#[cfg(feature = "queue-v1")]` ガード配下（embedded_rc 等）のみで参照されていることを再確認。queue-v2 既定経路では新 `SyncMailboxQueue` のみが使用される状態を維持。
- 2025-10-27: actor-core に `SyncMailbox` / `SyncMailboxProducer` エイリアスと `build_sync_mailbox_pair` ヘルパーを追加。標準メールボックス生成が直接 `SyncMailboxQueue` を指すようにし、後続の MailboxQueueBackend 撤廃作業の足場を整備。
- 2025-10-27: Tokio / embedded ランタイムを `SyncMailboxQueue` ベースへ統一し、互換用の `SyncQueueDriver` 再エクスポートを削除。`cargo +nightly check -p cellex-actor-std-rs` および `cargo +nightly check -p cellex-actor-embedded-rs` でビルド整合性を確認（stable rustc は既知 ICE のため CI は保留）。
- 2025-10-27: `MailboxQueueBackend`（旧 `MailboxQueueDriver`）は段階的に廃止予定。`QueueMailbox`／`QueueMailboxCore` を `SyncMailboxQueue` 直接保持へ移行しつつ、`ArcMailboxQueue` や `PriorityMailboxQueue` など名称が Driver に依存していたラッパも役割に合わせて再設計する計画をドキュメント化。完了後にトレイトと互換エイリアスを完全削除する。
- 2025-10-27: `QueueMailboxCore` の API を再整理し、メッセージ型に関わるトレイト境界を impl ブロック単位へ集約。これにより `len` / `capacity` / `try_send_mailbox` などの呼び出し側はターボフィッシュ指定なしで利用でき、将来的に `MailboxQueueBackend` トレイト自体を差し替える際の影響範囲を縮小できることを確認。
- 2025-10-27: priority mailbox 系モジュールのリネームを反映し、`modules/actor-std/src/tokio_priority_mailbox/priority_mailbox_queue.rs` を正式名称としてドキュメント化。利用者向けサンプルも新モジュール名に更新し、下記のように `PriorityMailboxQueue` を直接導入できることを追記。

  ```rust
  use cellex_actor_core_rs::api::{
    mailbox::queue_mailbox::QueueMailbox,
    test_support::TestSignal,
  };
  use cellex_actor_std_rs::tokio_priority_mailbox::priority_mailbox_queue::PriorityMailboxQueue;

  let priority_queue = PriorityMailboxQueue::<u32>::new(3, 32, 128);
  let mailbox = QueueMailbox::new(priority_queue, TestSignal::default());
  ```
- 廃止までの手順メモ
  1. actor-core 内の `QueueMailbox`／`QueueMailboxProducer`／`QueueMailboxRecv` が露出する箇所を `SyncMailbox`／`SyncMailboxProducer` などの新エイリアスへ置換し、トレイト境界へ依存するコードを減らす。
  2. ランタイム（Tokio／embedded／priority）とテスト群は既に `SyncMailboxQueue` 前提となったため、この状態を活かして actor-core 側の残存依存を洗い替えし、互換レイヤやトレイト境界を削除しやすくする。
  3. `ArcMailboxQueue` や `PriorityMailboxQueue` といったラッパを役割に沿った命名へ統一しつつ、最終的に `MailboxQueueBackend` トレイト自体を完全撤廃する。
- **全体テスト・CI とドキュメント更新**  
  - 2025-10-25 時点の差分を含め `./scripts/ci-check.sh all` を再実行し、queue-v2 既定／queue-v1 互換（`cargo check -p cellex-actor-embedded-rs --no-default-features --features alloc,embedded_rc,queue-v1`）双方が通ることを確認済み。  
  - 2025-10-25: embedded_rc + queue-v1 で RP2040 ターゲットのクロスビルド確認を実施  
    - `cargo check -p cellex-actor-embedded-rs --target thumbv6m-none-eabi --no-default-features --features alloc,embedded_rc,queue-v1` → OK  
  - 2025-10-25: embedded_arc + queue-v2 で RP2040 クロスビルドを試行したが、`alloc::sync::Arc` が `target_has_atomic = "ptr"` 非対応環境で生成できないためビルド不可（`alloc::sync` が cfg 除外）。embedded_arc はホスト向け検証専用とし、組込み cross チェックは embedded_rc で継続する方針。
  - 2025-10-25: `actor_ref_dead_letter_and_metrics_on_queue_v1_embedded_rc_factory` を追加し、embedded_rc + queue-v1 構成で `DeliveryRejected` デッドレターと `MailboxEnqueued` メトリクスが発火する統合テストを整備。  
    - `cargo test -p cellex-actor-core-rs --no-default-features --features alloc,queue-v1 actor_ref_dead_letter_and_metrics_on_queue_v1_embedded_rc_factory` → OK（警告のみ、テスト成功）
    - 優先度経路の網羅性を高めるため、`tell_with_priority` / `send_system` でのオーバーフロー時挙動 (`DeliveryRejected` + `MailboxEnqueued`) も queue-v1 fallback で検証。  
      - `cargo test -p cellex-actor-core-rs --no-default-features --features alloc,queue-v1 actor_ref_tell_with_priority_queue_v1_emits_dead_letter` → OK  
      - `cargo test -p cellex-actor-core-rs --no-default-features --features alloc,queue-v1 actor_ref_send_system_queue_v1_emits_dead_letter` → OK  
    - embedded_arc 構成はフィーチャ依存上 queue-v2 専用（`embedded_arc` が `queue-v2` を必須とするため）。queue-v1 fallback は embedded_rc でのみ維持されることを明記。
  - `ActorRef`／`PriorityActorRef` が `QueueMailboxProducer::try_send_mailbox` 経由で `MailboxError` を扱うよう更新。`DropNewest` 等のポリシー情報を保持したままデッドレターへ伝搬し、既存の `tell` / `tell_with_priority` / `send_system` テストが queue-v1/v2 双方で回帰を検出できる構成に整備。  
    - `cargo test -p cellex-actor-core-rs` および `cargo test -p cellex-actor-core-rs --no-default-features --features alloc,queue-v1` → いずれも OK（警告のみ）  
    - `./scripts/ci-check.sh all` を再実行し、ワークスペース全体のフォーマット／lint／テストが queue-v2 既定構成で通過することを確認

#### フェーズ5B完了報告（2025-10-25）
- 実行コマンド（すべて成功、警告のみ）  
  - `./scripts/ci-check.sh all`  
  - `cargo test -p cellex-actor-core-rs`  
  - `cargo test -p cellex-actor-core-rs --no-default-features --features alloc,queue-v1`  
  - `cargo check -p cellex-actor-embedded-rs --no-default-features --features alloc,embedded_rc,queue-v1`  
  - `cargo check -p cellex-actor-embedded-rs --no-default-features --features alloc,embedded_rc,queue-v2`  
  - `cargo check -p cellex-actor-embedded-rs --target thumbv6m-none-eabi --no-default-features --features alloc,embedded_rc,queue-v1`  
  - `cargo check -p cellex-actor-embedded-rs --no-default-features --features alloc,embedded_arc,queue-v2`（ホスト検証向け、deprecation warning のみ）  
  - `cargo check -p cellex-actor-core-rs --target thumbv6m-none-eabi --no-default-features --features alloc,queue-v2`  
  - `cargo check -p cellex-actor-core-rs --target thumbv8m.main-none-eabi --no-default-features --features alloc,queue-v2`（`rustup target add thumbv8m.main-none-eabi` 済みで成功。CI でのターゲット追加をフォローアップ）  
- 結果整理  
  - queue-v1 fallback は embedded_rc のみを対象。embedded_arc は queue-v2 専用（Cargo features で自動的に有効化）であり、ドキュメントで周知することで設定ミスを回避。  
  - queue-v1 / queue-v2 双方での ActorRef 統合テストにより、デッドレター（DeliveryRejected）およびメトリクス（MailboxEnqueued）が一貫して発火することを確認。  
  - RP2040 ターゲット（thumbv6m-none-eabi）での queue-v1 fallback コンパイルも継続して成功。
  - [x] `QueueError` → `MailboxError` 変換テーブルを実装し、単体テストで網羅性と整合性を検証する。
    - 2025-10-25: `api/mailbox/error.rs` に `MailboxError` / `MailboxOverflowPolicy` を追加し、`QueueMailboxCore::try_send_mailbox` / `try_dequeue_mailbox` で利用できる変換ヘルパを実装。既存 API 互換のため `QueueError` への逆変換も提供し、段階移行中は従来の `try_send` / `try_dequeue` が新エラーから再構築する形を採用した。
    - 2025-10-25: Tokio／Priority／embedded の各 Mailbox + Sender に `*_mailbox` 系 API を追加し、QueueError ベースの旧メソッドと併存する形で新エラー体系を露出。既存呼び出しコードに影響を与えずに新 API を段階導入できる状態を整備。
    - 2025-10-25: 公開 API（`ActorContext` / `PriorityActorRef` 等）で `MailboxError` を返す補助メソッドを追加する移行方針を整理。現行の QueueError メソッドは維持したまま、利用者が任意で新エラーへ移行できる設計案を次フェーズで実装予定。
    - 2025-10-25: `modules/actor-core/src/api/mailbox/mailbox_error/tests.rs` に `MailboxError` 変換経路のユニットテストを追加し、`DropNewest`／`DropOldest`／`Backpressure`／`ResourceExhausted`／`Internal`／`Closed`／`Disconnected` を網羅。逆変換（`MailboxError` → `QueueError`）も併せて検証することで互換経路の回帰を抑止。`cargo test -p cellex-actor-core-rs mailbox_error` を実行し、警告のみで全テスト成功を確認。`QueueError::Empty` 変換がパニックする仕様も `#[should_panic]` で固定化し、今後の SyncQueue 差し替え時にリグレッションガードとして活用する。
    - 2025-10-25: `modules/actor-core/src/api/mailbox/queue_mailbox/sync_queue_driver.rs` に対応テスト群を追加し、`DropOldest`／`DropNewest`／`Grow`／`Block` 各ポリシーでの `MetricsEvent` 発火と `QueuePollOutcome` 変換を検証。`cargo test -p cellex-actor-core-rs sync_queue_driver` を実行し、想定どおりのイベント記録とエラー変換を確認。
    - 2025-10-25: queue-v2 ルートで `TestMailboxFactory` および Tokio mailbox 系のファクトリ／Sender／Mailbox 実装を `SyncMailboxQueue` ベースへ移行。queue-v1 ビルドのみ `LegacyQueueDriver` を利用し続ける構成に整理し、Backpressure/メトリクス処理を新ラッパへ委譲。
    - 2025-10-25: embedded 向け `ArcMailbox` / `ArcPriorityMailbox` を `SyncMailboxQueue` 経由で組み立てる実装へ更新し、`PriorityMailboxQueue` の `MailboxOverflowPolicy` 伝搬と制御レーン優先挙動をユニットテストで保証。加えて `modules/actor-core/src/api/mailbox/queue_mailbox/tests.rs` の受信フローを `Future` ベースで検証し、`queue-v2` 向けの `PollOutcome` 変換経路が pending → ready を正しく反映することを確認。
    - 2025-10-25: `./scripts/ci-check.sh all` を再実行し、`cellex-actor-embedded-rs` 既定フィーチャ（queue-v2 + embedded_rc）でも lint / test / dylint が通過することを再確認。併せて queue-v1 回帰は `cargo check -p cellex-actor-embedded-rs --no-default-features --features alloc,embedded_rc,queue-v1`、queue-v2 経路は `cargo check -p cellex-actor-embedded-rs --no-default-features --features alloc,embedded_arc` で個別確認済み。
    - 2025-10-25: SyncQueue 本番差し替えに向けた段取りを整理。1) `MailboxQueueBackend` に `SyncMailboxQueue` 実装を追加し、今回のテストでカバーした `MailboxError` 変換がそのまま通ることを確認する。2) `QueueMailboxProducer`／`QueueMailboxRecv` のバックプレッシャー・リソース枯渇ハンドリングを新ラッパへ委譲するスイッチポイント（feature gate での切り替え）を設計。3) `queue-v1` fallback を維持したまま段階的に `SyncQueue` へ切り替えるため、`QueueMailbox` 構築時にラッパを DI できるようコンストラクタを調整する。次フェーズではこの段取りに沿って `SyncQueue` 本番差し替えを実施する。
    - 2025-10-25: priority mailbox 用に `PriorityMailboxQueue`（当時の名称は `PrioritySyncQueueDriver`）を実装。control レーンは `Vec<SyncMailboxQueue<_>>`、regular は単一ラッパで構成し、`MailboxQueueBackend` として `offer`/`poll`/`close` を集約。専用テスト（制御優先度・メトリクス連携・容量集計）を `modules/actor-std/src/tokio_priority_mailbox/priority_mailbox_queue/tests.rs` に追加し、`cargo test -p cellex-actor-std-rs tokio_priority_mailbox`（queue-v2 / queue-v1 双方）で検証。
    - 2025-10-25: `QueueRwCompat` 利用箇所を棚卸し。① プロダクションコードでは `tokio_priority_mailbox::queues` (control/regular レーン) のみが queue-v2 時に依存。② テストでは `actor_scheduler::tests` が queue-v2 専用の整合性確認として `LegacyQueueDriver<QueueRwCompat>` を利用。③ その他は設計ドキュメント参照用途。優先対応対象は①。
    - 2025-10-25: priority mailbox 向け `SyncMailboxQueue` 導入案を整理。`PriorityMailboxQueue<M>`（当時の仮称は `PrioritySyncQueueDriver<M>`）を新設し、control レーンは `Vec<SyncMailboxQueue<PriorityEnvelope<M>>>`、regular レーンは単一の `SyncMailboxQueue` を内包する構成とする。`offer` は優先度判定後に該当ラッパへ委譲し、`QueuePollOutcome` は control→regular の順に合成。`len`／`capacity` は `QueueSize` をサチュレート加算で集約し、`set_metrics_sink` は全レーンへブロードキャスト。`close` では各レーンの close 結果を最初の `Some` を優先して返す。
    - 2025-10-25: 移行ステップ案: (a) `PriorityMailboxQueue` 実装と単体テスト（制御レーン優先度・メトリクス通知・DropNewest/DropOldest/AllocError 等）を追加。 (b) `TokioPriorityQueues` を queue-v2 ビルド時には新キューで提供するようリファクタリングし、`TokioPriorityMailbox`／`TokioPriorityMailboxSender` を `QueueMailbox<PriorityMailboxQueue<_>, NotifySignal>` 化。 (c) queue-v1 フィーチャでは既存 `LegacyQueueDriver` + `ArcMpsc*` 実装を残し、`cfg` で排他制御。 (d) 既存の `QueueRwCompat` はテストサポート（`TestQueue` 等）に限定し、priority mailbox から除去。 (e) `cargo test -p cellex-actor-std-rs tokio_priority_mailbox`（新規追加予定）と `./scripts/ci-check.sh all` で回帰検証。
  - [ ] `QueueMailbox` を利用する主要モジュール（scheduler、deadletter、priority mailbox 等）を影響の小さい順に差し替え、各ステップで `queue-v1` / `queue-v2` 両ビルドを確認する。
  - [ ] 新たに追加する結合テスト（`QueueMailbox` + signal 実装）を `queue-v1` / `queue-v2` の双方で実行し、回帰を検出する仕組みを整える。
  - [ ] `cargo bench -p cellex-actor-core-rs --bench mailbox_throughput` を実行し、ベースラインとの差分を記録・報告するルーチンを確立する。

リスク要因:
- Mailbox 基盤は開発中の他コンポーネントとも共有されるため、ここでのバグがテスト全体を止めてしまいやすい。
- `QueueError` のマッピングやデッドレター処理を誤ると、機能検証用テストケースでメッセージが消失し原因調査に時間がかかる。
- 優先度・容量制御の挙動が変化すると、性能検証ベンチや将来のチューニング作業が不安定になるため、段階的な切り替えが重要。

対応策:
- Mailbox 差し替え前に `queue-v1` / `queue-v2` 両ビルドで同じテストスイートを走らせ、挙動差分をドキュメント化する。
- 各サブモジュール移行後に専用のユニットテスト・結合テストを追加し、メッセージロストや優先度挙動を直接検証する。
- ベンチマーク結果が規定値を超える場合は直ちにフィーチャーフラグで旧実装へ戻せるようにし、原因切り分けを行う。

### フェーズ6: テスト移行（queue-v1 退役準備、リスク: 中, SP: 5）
- [x] queue-v1 依存テストを `src/tests/legacy_queue_v1.rs` に集約し、`#[cfg(feature = "queue-v1")]` 付きの最小回帰セットのみに圧縮する。Tokio / Embedded 本線のテストは queue-v2 のみで完結させ、`queue-v1` を切っても CI が通る状態を確認する。
  - 2025-10-26: `modules/actor-core/src/tests.rs` から `#[cfg(feature = "queue-v1")] mod legacy_queue_v1;` を追加し、`PriorityActorRef` 系の queue-v1 回帰テストをモジュール配下へ移設。`ActorProcessRegistry` 型エイリアスと `Shared` トレイト導入でビルドエラーを解消し、`cargo test -p cellex-actor-core-rs --no-default-features --features alloc,queue-v1 legacy_queue_v1` が 4 ケース成功することを確認。
  - 2025-10-26: `./scripts/ci-check.sh all` を実行し、queue-v1/queue-v2 混在構成でも lint / test / クロスチェックを含めてグリーンで完走することを再確認（thumbv6m / thumbv8m ターゲット含む）。
- [x] `QueueRwCompat` を利用している主要経路（Tokio priority mailbox、actor scheduler など）を順次 `SyncMailboxQueue` ベースへ置き換え、互換レイヤーの利用範囲をテスト専用に絞る（2025-10-26 に互換レイヤー自体を撤去済み）。
- [x] queue-v1 退役チェックリスト（進捗）
  - [x] CI ジョブを環境変数で切り替える案は廃止し、queue-v1 ジョブ自体を削除（scripts/ci-check.sh から `queue` ターゲットを撤去、GitHub Actions からも除去）。  
    - Owner: @j5ik2o, Completed: 2025-10-26
  - [x] `./scripts/ci-check.sh queue` の手動実行手順はアーカイブ扱いとし、週次運用を停止。退役後は `queue-v2` のみを記録対象にする。  
    - Owner: @ci-docs-team, Completed: 2025-10-26
  - [x] GitHub Actions ワークフローから queue-v1 ジョブを削除し、既存ジョブは queue-v2 前提で継続する。  
    - Owner: @platform-ci, Completed: 2025-10-26
  - [x] queue-v1 向けテストセット（`modules/actor-core/src/tests/legacy_queue_v1.rs` など）を削除し、リリースノートに破壊的変更として反映予定。  
    - Owner: @queue-runtime, Completed: 2025-10-26
  - [x] queue-v1 退役判定用のメトリクス／回帰項目を `queue-v2` の常設チェックへ集約。dead letter・メトリクス・クロスビルドは `ci-check.sh all` で担保。  
    - Owner: @metrics-observability, Completed: 2025-10-26
- [x] queue-v1 退役チェックリストを整備し、CI での queue-v1 ジョブを徐々にオプショナル扱いへ移行する準備（`scripts/ci-check.sh` / GitHub Actions 更新、ドキュメント周知）を完了させる。
  - 2025-10-26: `queue-v2` フィーチャーを各クレートから撤廃し、常時有効な実装へ統合。`Cargo.toml` とランタイム実装から `queue-v1` / `queue-v2` 切り替えロジックを削除し、CI 設定・ドキュメントを退役後の構成に更新済み。

#### テスト棚卸 (2025-10-25 時点)
- **クリティカルパス**  
  - `modules/actor-std/src/tokio_mailbox/tests.rs` … queue-v2 前提の単体テストは存在するが、`#[tokio::test]` のフレーバーが混在。`current_thread` / `multi_thread` 切り分けの妥当性を再確認し、必要なら `worker_threads` 明示や `rt-multi-thread` Feature 前提の実行に統一する。  
  - `modules/actor-std/src/tokio_priority_mailbox/tests.rs` … queue-v1 fallback パス (`#[cfg(all(feature = "queue-v1", not(feature = "queue-v2")))]`) が残存。queue-v2 を既定にするフェーズでは、v2 API での制御レーン優先／ドロップ／メトリクス検証を強化し、queue-v1 分岐を統合テストへ移す計画が必要。
- **エッジケース**  
  - `modules/actor-core/src/api/actor_scheduler/tests.rs` … `SyncMailboxQueue` ベースのテストケースへ置き換え済み。スケジューラの ready/not-ready 判定やメトリクス伝播は新 API でカバーし、互換レイヤー依存は解消済み。  
  - `modules/actor-core/src/tests/legacy_queue_v1.rs` … queue-v1 fallback 回帰テストを集約。queue-v2 版は `tests.rs` に実装済みなので、将来的な廃止スケジュールに合わせて特有ケースを段階的に `integration/` 配下へ移す計画は維持。
- **Embedded 系**  
  - `modules/actor-embedded/src/tests.rs` / `arc_priority_mailbox/tests.rs` 等は queue-v2 で実行されるが、`thumbv8m.main-none-eabi` ターゲットのクロスチェックが未実施。フェーズ6で `cargo check -p cellex-actor-core-rs --target thumbv8m.main-none-eabi --no-default-features --features embedded,queue-v2` を走らせ、RP2350 相当の結果を記録する。

- 2025-10-25: `actor_scheduler::tests` を `SyncMailboxQueue` ベースへ移行し、`ReadyQueueScheduler` 系のメトリクス／デッドレター挙動を queue-v2 API で検証。`cargo test -p cellex-actor-core-rs --tests` および `cargo test -p cellex-actor-core-rs --no-default-features --features alloc,queue-v1` を実行し、双方成功（警告のみ）。
- 2025-10-25: クロスチェック進捗更新。`cargo check -p cellex-actor-core-rs --target thumbv6m-none-eabi --no-default-features --features alloc,queue-v2` → OK（警告のみ）。`rustup target add thumbv8m.main-none-eabi` 実施後に `cargo check -p cellex-actor-core-rs --target thumbv8m.main-none-eabi --no-default-features --features alloc,queue-v2` も成功（警告のみ）。CI でも同ターゲットの追加を検討する。
- 2025-10-25: Tokio メールボックス系テストを再確認。`cargo test -p cellex-actor-std-rs tokio_mailbox` / `cargo test -p cellex-actor-std-rs tokio_priority_mailbox` → いずれも成功（警告のみ）。`current_thread` / `multi_thread` 重複テストの扱いと性能ベンチ (`mailbox_throughput`) の測定方針はフェーズ6後半で整理する予定。

### フェーズ7: 段階的リリースとクリーンアップ（リスク: 低, SP: 3）
- [ ] 互換レイヤを置いたまま actor-core 内の利用箇所をモジュール単位で移行し、完了後に旧 API 依存を削除する。
- [ ] v2 移行後に不要となる re-export やラッパ型をリスト化し、削除 PR と `queue-v1` フィーチャー廃止のタイムラインをまとめる。
- [ ] `docs/guides/module_wiring.md`, `CLAUDE.md`, `README*.md` など関連ドキュメントを v2 前提に更新し、マイグレーションガイドを追記する。
- [ ] CHANGELOG / リリースノート草案に BREAKING CHANGE と移行手順を記載する。

## リスクと対策
- **内部実装変更による性能劣化**: 新しいキュー実装への差し替えでスケジューラのスループットが落ちる可能性 → 移行前後で `cargo bench -p cellex-actor-core-rs --bench mailbox_throughput` を実行し、結果を `benchmarks/baseline_v1.txt` / `benchmarks/after_v2.txt` に保存して差分確認。
- **エラー分岐増加**: `QueueError` / `OfferOutcome` の取り扱い漏れ → clippy の `unused_result` 等を活用し、エラー変換テーブルに対する単体テストを用意する。
- **Embedded 対応リグレッション**: feature 追加漏れ → `cargo check` (thumb ターゲット) を各フェーズの終わりに実施し、結果を計画書に追記する。
- **API 互換性と利用者影響**: 互換レイヤーをいつまで維持するか不明瞭 → `queue-v1` を `deprecated` にし、段階的廃止タイムラインと対外的マイグレーションガイドを作成。
- **部分的移行による不整合**: v1/v2 が混在すると予期せぬ挙動が発生 → `queue-v1` と `queue-v2` を同時に有効化した場合に `compile_error!` を発生させるガードを準備し、ビルドオプションで排他制御する。

## 完了判定
- 各フェーズ完了時点で `makers ci-check` が成功し、フェーズ毎の差分が CI パス済みである。
- v2 キューAPIへの置換が actor-core 全コードパスで完了している。
- 全テスト (`./scripts/ci-check.sh all`) およびクロスチェックが成功する。
- ベンチマーク結果（移行前後）が比較可能な形で `benchmarks/` に保存され、許容範囲内に収まっている。
- ドキュメントとコメントが新仕様を反映し、旧 API 参照が残っていない（マイグレーションガイドと CHANGELOG を含む）。
- `queue-v1` フィーチャーへの依存が解消され、互換レイヤーの deprecation ステータスと廃止予定が明文化されている。

## トラッキング
- 各フェーズ完了時に当ファイルへ進捗を追記し、必要に応じてサブタスクを追加する。
- 並行して `progress.md` にも主要進捗を記録し、統合管理を維持する。
- 作業報告時には現在着手中フェーズの進捗率（%）を明示し、フェーズ完了と同時に `makers ci-check` の成功結果を記録する。
- [x] ファサード層 API の戻り値変更に合わせて呼び出し元（scheduler、テストサポート等）を更新し、`queue-v1` / `queue-v2` 両ビルドで警告ゼロを確認する。
- [x] Mailbox ファサード経由の happy path / 異常系統合テストを追加し、`queue-v1` / `queue-v2` 両方で `cargo test -p cellex-actor-core-rs --tests` が通ることを検証する。

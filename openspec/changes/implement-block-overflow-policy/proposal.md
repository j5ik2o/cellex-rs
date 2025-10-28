# Proposal: implement-block-overflow-policy

**Status:** Draft  
**Created:** 2025-10-28  
**Author:** Claude (AI Assistant)

## Why

- `OverflowPolicy::Block` を指定した Mailbox 経路では、依然として同期キュー (`SyncQueue`/`VecRingBackend`) を通るため即座に `QueueError::Full` を返してしまい、ドキュメント記載の「空きができるまで非同期待機」と乖離している。
- `AsyncQueue` と `SyncAdapterQueueBackend` には既に FIFO 待機を備えた実装が存在するが、Mailbox 経路が同期 API のままなので活用されていない。
- 参照実装（protoactor-go, pekko）はいずれもバウンデッド Mailbox で producer をブロックする。プロジェクト方針でも実装との不整合を解消することが求められている。

## What Changes

- `MailboxQueueBackend` トレイトを `async_trait` ベースに刷新し、`offer` / `poll` / `close` を非同期 API へ変更する。
- `UserMailboxQueue` を `AsyncQueue<EntryShared<M>, SyncAdapterQueueBackend<VecRingBackend>>` 上に再構築し、`AsyncQueue::offer_shared` 内部の待機処理をそのまま利用する。
- `QueueMailboxCore`／`QueueMailboxHandle`／`QueueMailboxProducer` 等、Mailbox 経路全体を `async fn` 化し、呼び出し側に `await` 伝播する。
- メトリクス／テスト／ドキュメントを async 化に合わせて更新し、`OverflowPolicy::Block` が実際に待機する振る舞いを検証する。
- **Breaking**: Mailbox API が非同期化されるため、呼び出し側も `await` 対応が必須になる。

## Impact

- **Affected specs**: mailbox-overflow-handling, mailbox-core-api
- **Affected crates**:
  - `modules/actor-core/src/api/mailbox/queue_mailbox/backend.rs`（トレイトの async 化）
  - `modules/actor-core/src/api/mailbox/queue_mailbox/{core.rs,handle.rs,producer.rs}`（呼び出し経路の async 化）
  - `modules/actor-core/src/api/mailbox/queue_mailbox/user_mailbox_queue.rs`（AsyncQueue への置き換え）
  - `modules/utils-core/src/collections/queue/backend/sync_adapter_queue_backend.rs`（補助 API の微調整想定）
  - 影響するテスト・サンプル（`actor-core` 内の Mailbox テスト群 等）
- **Breaking Changes**:
  - Mailbox 経路の同期 API が削除され、`await` が必須
  - `MailboxQueueBackend::offer` が `Result<OfferOutcome, QueueError<M>>` のまま非同期化されるため、呼び出し側テストの書き換えが必要

## Motivation

### 現状の問題整理

1. **Mailbox が同期キュー依存のまま**
   - `UserMailboxQueue` は `SyncQueue<VecRingBackend>` を使用し、`QueueMailboxCore::try_send_mailbox` も同期的に呼び出している。
   - `OverflowPolicy::Block` を選択しても、`VecRingBackend` 上では `Err(QueueError::Full(..))` を即座に返すのみで待機しない。

2. **既存の待機メカニズムが活用されていない**
   - `AsyncQueue::offer_shared` は `prepare_producer_wait` を通じて `WaitQueue` を利用し、Block 時に正しく suspend する。
   - `SyncAdapterQueueBackend` も Block ポリシー時に待機ハンドルを登録できるが、Mailbox 経路では呼び出されていない。

3. **ドキュメントと実装の乖離**
   - `docs/design/mailbox_expected_features.md` では Block が想定動作どおりにブロックすることをゴールとしている。
   - 参照実装との差異により、期待挙動が満たされていない。

### 選択肢の比較

- **Option A（推奨）**: Mailbox を AsyncQueue ベースに刷新し、既存の非同期待機を活用する。
- Option B: SyncQueue にスレッドブロックを導入 → 組み込み環境との非整合で却下。
- Option C: 仕様を「Block=即エラー」に書き換える → ドキュメント不整合を解消できず却下。

## Goals

1. Mailbox 経路において `OverflowPolicy::Block` が確実に非同期待機することを保証する。
2. `AsyncQueue` + `SyncAdapterQueueBackend` の既存待機機構を再利用し、新規ロジックを最小化する。
3. Mailbox API を async 化しつつ、Drop/Grow など他ポリシーの挙動は維持する。
4. テスト・ドキュメント・メトリクスを新しい API へ更新し、回帰を防止する。

### Non-Goals

- SyncQueue/VecRingBackend にスレッドブロッキング実装を追加すること。
- Block 以外の OverflowPolicy アルゴリズムを変更すること。
- Mailbox のスケジューラ連携ロジック（ReadyQueue 等）の新機能開発。

## Proposed Changes

### 1. MailboxQueueBackend の async 化

- `#[async_trait(?Send)]` を付与し、以下のメソッドを `async fn` に変更:
  - `offer`, `poll`, `close`, `len`, `capacity`, `set_metrics_sink` は従来どおり同期で良いが、`offer`/`poll`/`close` は `await` 可能にする。
- 既存実装（`UserMailboxQueue`, `SystemMailboxQueue` 系, テストダブル）を全て更新。

### 2. UserMailboxQueue の AsyncQueue 化

- `VecRingBackend<EntryShared<M>>` を `SyncAdapterQueueBackend` で包み、`AsyncQueue` にマウントする。
- `offer` / `poll` / `close` などの MailboxQueueBackend 実装を `async fn` に置き換え、`self.queue.offer(..).await` を利用。
- `ArcShared<SpinAsyncMutex<SyncAdapterQueueBackend<..>>>` を内部共有状態として確保。

### 3. Mailbox Core/Handle/Producer の async 化

- `QueueMailboxCore::try_send_mailbox`/`try_send`/`try_dequeue_mailbox` を `async fn` 化し、内部呼び出しで `await`。
- `QueueMailbox` の公開 API (`offer`, `poll`, `flush`, `close` 等) を順次 `async fn` 化。呼び出し側のテスト・サンプルも更新。
- `MailboxSignal` インターフェースとの整合を確認（必要であれば `Future` 化）。

### 4. テストと検証

- `actor-core/src/api/mailbox/queue_mailbox/tests.rs` 等で Block ポリシーが待機する統合テストを再構築。
- `AsyncQueue` レベルの既存テストを再利用しつつ、Mailbox 経由での待機確認を追加。
- クロスコンパイル／CI チェックを async 化後に再実行。

### 5. ドキュメント更新

- `docs/design/mailbox_expected_features.md` の Block 項目を ✅ に更新。
- API リファレンス（rustdoc コメント）は英語、その他ドキュメントは日本語で async 化後の使用方法を説明。

## Impact Analysis

- **API Breaking**: Mailbox を利用する全呼び出し元が `await` への書き換えを要する。
- **テスト更新**: 既存の同期テストは `tokio::test` など async テストへ移行する。
- **組み込み互換性**: `#[async_trait(?Send)]` と `SpinAsyncMutex` により `no_std` / `embassy` 環境でも動作する設計を維持。
- **パフォーマンス**: 非同期化に伴うオーバーヘッドはタスク切り替え程度であり、満杯時の fairness 向上メリットが勝る。

## Migration Strategy

1. `MailboxQueueBackend` トレイトを async 化し、コンパイルエラーで影響範囲を炙り出す。
2. `UserMailboxQueue` を AsyncQueue に移行、`QueueMailboxCore` を async 化。
3. 他コンポーネント（例: `QueueMailboxHandle`, `QueueMailboxProducer`）を順次 async 対応。
4. テスト・examples・bench を async 版へ書き換え。
5. `docs` と `openspec` の更新を反映。

## Alternatives Considered

### Option B: SyncQueue に条件付き待機を追加

- スレッドブロッキングが必要で組み込み環境と相性が悪いため不採用。

### Option C: Block=即エラー としてドキュメント修正

- 本質的な期待値を満たさず、参照実装との乖離が残るため不採用。

## Dependencies

- `async-trait` クレート（既に依存済み）
- `AsyncQueue` / `SyncAdapterQueueBackend` / `WaitQueue` 既存実装
- Mailbox 周辺の tokio テスト基盤

## Success Metrics

1. Mailbox 経由で `OverflowPolicy::Block` を使用した場合に producer が待機し、consumer `poll` 後に `await` が完了する。
2. Block 以外の OverflowPolicy で従来テストがすべてパスする。
3. `./scripts/ci-check.sh all` と `openspec validate implement-block-overflow-policy --strict` が成功する。
4. `docs/design/mailbox_expected_features.md` の Block 項目が ✅ へ更新される。

## Open Questions

1. `QueueMailbox` API 全体を非同期化した場合、呼び出し元（例: スケジューラ）のスレッドモデルに追加対応が必要か？
2. `MailboxSignal` インターフェースとの同期/非同期整合をどのように確保するか？（現在の `notify` が同期 API のままでも十分か）
3. Block 待機が長時間継続するケースでメトリクスやタイムアウトの追加が必要か？

## References

- `docs/design/mailbox_expected_features.md`
- `docs/sources/protoactor-go/actor/bounded.go`
- `docs/sources/pekko/docs/src/main/paradox/mailboxes.md`
- `modules/utils-core/src/collections/queue/async_queue.rs`
- `modules/utils-core/src/collections/queue/backend/sync_adapter_queue_backend.rs`

# Tokio 向け AsyncQueue 利用ガイド

## 概要

`modules/utils-std/src/v2/collections/async_queue/` では、Tokio ランタイム上で動作する
非同期 MPSC キューを簡単に構築できるユーティリティを提供しています。本ガイドでは
`make_tokio_mpsc_queue` を利用した基本的な使い方と注意点をまとめます。

## 前提条件

- Tokio 1.40 以降を利用していること（`workspace` では `tokio` クレートを既に依存設定済み）
- `utils-core` / `utils-std` v2 コレクションをプロジェクトに取り込んでいること

## キューの構築と利用

```rust
use utils_std::v2::collections::async_queue::make_tokio_mpsc_queue;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // バッファ容量 1024 の MPSC キューを構築
  let queue = make_tokio_mpsc_queue::<String>(1024);

  // 複数プロデューサからの offer が可能
  let producer = queue.producer_clone();
  tokio::spawn(async move {
    producer.offer("hello".to_owned()).await.unwrap();
  });

  // コンシューマは await でメッセージを受信
  let message = queue.poll().await?;
  assert_eq!(message, "hello");

  queue.close().await?;
  Ok(())
}
```

### 特徴

- `TokioBoundedMpscBackend` が内部で `tokio::sync::mpsc::channel` をラップし、溢れ時は
  `WaitQueue` を用いたバックプレッシャ制御を行います。
- `AsyncQueue` を通じて offer/poll/close 等、既存 v2 API と同じ操作感で利用できます。
- `close()` 呼び出し後は新規 offer が `QueueError::Closed` で失敗し、待機中の Future
  は同エラーで解放されます。

## テストのヒント

- キュー溢れ時の待機を検証する場合は、`queue.clone().offer(value)` を `tokio::select!` で
  包み、一定時間内に完了しないことを確認すると安定します。
- `#[tokio::test(flavor = "multi_thread")]` を利用すると、待機が `Send` な Future でも正しく
  スケジューリングされます。

## 参考リンク

- `modules/utils-std/src/v2/collections/async_queue/tokio_bounded_mpsc_backend.rs`
- `modules/utils-std/src/v2/collections/async_queue/tests.rs`
- `docs/guides/v2_queue_migration.md`

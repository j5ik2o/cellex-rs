# Embedded Runtime High-Level API Plan (2025-10-08)

## 現状サマリ (2025-10-13)
- `ActorSystemRunner` / `ShutdownToken` を中心にしたランナー構成は actor-core に統合済みで、embedded 環境でも `run_until_idle()` を呼び出せば動作する。
- `actor-embedded` には `ImmediateSpawner` / `ImmediateTimer` / `LocalMailboxRuntime` が揃っており、`embedded_rc` / `embedded_arc` 向けに MailboxRuntime を切り替え可能。
- `ActorRuntimeBundleEmbassyExt` により Embassy スケジューラへの差し替えフックが提供されている。

## 未解決課題
- [MUST] ランナーを常駐タスクとして扱う `SystemDriver` 抽象（Tokio/Embedded 共通）を実装していない。現在は利用者が直接 `run_until_idle()` を呼ぶ必要がある。
- [SHOULD] Embedded 向けの標準サンプル／ガイドが未整備で、メインループへの統合手順や停止フローがドキュメント化できていない。
- [MUST] Embassy 向け ReceiveTimeoutDriver が未実装のため、`embassy_executor` でのタイムアウト動作が PoC 止まり。
- [MUST] `embedded_arc` / `embedded_rc` のクロスビルドやランタイムテストを CI に組み込めておらず、レグレッション検出が手作業になっている。

## 優先アクション
1. `SystemDriver`（Tokio/Embedded 実装）の設計を固め、`ActorSystem::into_runner()` から利用できる公式 API として整備する。
2. RP2040 / RP2350 向けの最小サンプルを作成し、`run_until_idle()` の使い方と停止フローを README / CLAUDE.md に追記する。
3. `EmbassyReceiveTimeoutDriver` を実装し、`with_embassy_scheduler()` と組み合わせた統合テスト（またはハードウェア無しのシミュレーション）を準備する。
4. `cargo check -p nexus-actor-embedded-rs --no-default-features --features alloc,embedded_rc` などを CI に追加し、ビルド保証を自動化する。

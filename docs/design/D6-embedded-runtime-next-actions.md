# Embedded Runtime 高レベル API：次アクション

## 優先タスク
1. `SystemDriver` 抽象を設計・実装し、Tokio / Embedded 双方でランナーを常駐タスクとして扱えるようにする。現状は `run_until_idle()` 呼び出しを利用者が直書きしている。
2. RP2040 / RP2350 向け最小サンプルを追加し、埋め込み環境での停止フローをドキュメント化する。
3. `EmbassyReceiveTimeoutDriver` を実装し、`with_embassy_scheduler()` と組み合わせたテストを準備する。
4. `embedded_rc` / `embedded_arc` のクロスビルドとランタイムテストを CI に追加する。

## 参考
- 旧メモは `docs/design/archive/2025-10-08-embedded-runtime-plan.md` を参照。

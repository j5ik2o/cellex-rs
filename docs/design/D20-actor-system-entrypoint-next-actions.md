# ActorSystem エントリポイント簡素化：次アクション

## 優先タスク
1. `actor-std` に高レベルファサード（例: `TokioActorSystem::new`）を実装し、Guardian Props 指定から起動までを単一 API で完結させる。
2. `root_context.spawn` を段階的に非推奨化し、新 API への移行を促す（現状 `pub` のままで deprecation 未設定）。
3. サンプル・README を新しいエントリポイントに合わせて更新し、旧手順のサポートを整理する。
4. Embedded 向けにも同等のファサードが必要か検討し、必要なら設計案をまとめる。

## 参考
- 旧メモは `docs/design/archive/2025-10-15-actor-system-entrypoint-plan.md` を参照。

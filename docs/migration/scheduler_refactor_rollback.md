# ActorScheduler リファクタリング ロールバック手順書

**バージョン**: 1.0
**最終更新**: 2025-10-22
**対象フェーズ**: Phase 0 - Phase 4
**作成者**: QA / SRE チーム

## 1. 概要

このドキュメントは ActorScheduler リファクタリング各フェーズで問題が発生した際のロールバック手順を定義します。

### 1.1 ロールバックが必要なケース

- **パフォーマンス劣化**: ベンチマークで 5% 以上の劣化が検出された場合
- **安定性問題**: クラッシュ、デッドロック、メモリリークが本番環境で発生
- **互換性問題**: 既存アクターコードとの非互換により動作不良
- **重大なバグ**: データ損失やアクター停止など致命的な不具合
- **デプロイ失敗**: CI/CD パイプラインでの継続的な失敗

### 1.2 ロールバック判断基準

| 深刻度 | 条件 | 対応時間 | ロールバック判断者 |
|--------|------|----------|-------------------|
| **Critical** | サービス停止、データ損失 | 即座 | オンコールエンジニア |
| **High** | パフォーマンス劣化 >10%、頻繁なクラッシュ | 1時間以内 | テックリード |
| **Medium** | パフォーマンス劣化 5-10%、散発的な不具合 | 24時間以内 | プロジェクトマネージャー |
| **Low** | マイナーな非互換、ドキュメント不備 | 1週間以内 | 開発チーム判断 |

## 2. フェーズ別ロールバック手順

### Phase 0: 設計・プロトタイプ段階

**対象ブランチ**: `feature/ready-queue-coordinator-poc`

#### ロールバック手順

Phase 0 は main ブランチにマージされていないため、ロールバックは不要。

**問題発生時の対応**:
1. ブランチを削除またはアーカイブ
2. 設計ドキュメントをレビューし問題点を分析
3. ADR で設計変更を記録
4. 新しいアプローチで再設計

---

### Phase 1: ReadyQueueCoordinator 統合

**対象ブランチ**: `refactor/ready-queue-coordinator`
**マージ先**: `main`
**Feature Flag**: `new-scheduler` (デフォルト無効)

#### 前提条件

- [ ] Feature flag `new-scheduler` がデフォルトで無効化されている
- [ ] 既存の `ReadyQueueScheduler` が並行稼働可能
- [ ] ベンチマークベースラインが保存されている

#### ロールバック手順

##### ステップ 1: Feature Flag 無効化（即座）

```bash
# 1. Cargo.toml の feature flag 確認
grep -r "new-scheduler" Cargo.toml

# 2. デフォルト features から new-scheduler を除外
# Cargo.toml
[features]
default = ["std"]  # new-scheduler を含めない
new-scheduler = []
```

```rust
// 3. コード内で feature flag が正しく使われているか確認
#[cfg(feature = "new-scheduler")]
use ready_queue_coordinator::ReadyQueueCoordinator;

#[cfg(not(feature = "new-scheduler"))]
use ready_queue_scheduler::ReadyQueueScheduler;
```

##### ステップ 2: Git によるコード巻き戻し（1時間以内）

```bash
# 1. Phase 1 マージコミットを特定
git log --oneline --grep="Phase 1" --grep="ReadyQueueCoordinator" -i

# 2. マージコミットの直前にrevert
git revert -m 1 <merge-commit-hash>

# 3. revert コミットをpush
git push origin main

# 4. CI が green になることを確認
```

##### ステップ 3: デプロイとモニタリング（2時間以内）

```bash
# 1. ロールバック版をデプロイ
./scripts/deploy.sh production --version=rollback-phase1

# 2. メトリクスを監視
# - アクタースループット
# - メッセージレイテンシ
# - CPU/メモリ使用率
# - エラーレート

# 3. ログで異常がないか確認
kubectl logs -f deployment/actor-system --tail=1000 | grep -i error
```

#### 検証チェックリスト

- [ ] Feature flag が無効化されている
- [ ] 既存の ReadyQueueScheduler が使用されている
- [ ] ベンチマークがベースライン水準に戻っている（±2%以内）
- [ ] 本番環境でエラーレートが正常範囲内
- [ ] 全テストがパスしている

---

### Phase 2A: WorkerExecutor 導入

**対象ブランチ**: `refactor/worker-executor`
**Feature Flag**: `new-scheduler` (継続使用)

#### ロールバック手順

##### ステップ 1: Feature Flag 無効化

Phase 1 と同様に `new-scheduler` を無効化。

##### ステップ 2: Git revert

```bash
# Phase 2A マージコミットを revert
git log --oneline --grep="Phase 2A" --grep="WorkerExecutor" -i
git revert -m 1 <merge-commit-hash>
git push origin main
```

##### ステップ 3: ワーカ設定の復元

```toml
# config/actor_system.toml（Phase 1 の設定に戻す）
[scheduler]
type = "ready-queue-coordinator"  # Phase 2A 以前の設定
worker_count = 4
throughput = 32
```

#### 追加の復旧手順

- **ワーカタスクのクリーンアップ**: 停止したワーカタスクがリソースリークしていないか確認
- **メッセージキューの確認**: 未処理メッセージが残っていないかチェック

---

### Phase 2B: MessageInvoker 分離

**対象ブランチ**: `refactor/message-invoker`
**Feature Flag**: `new-scheduler`

#### ロールバック手順

##### ステップ 1: Feature Flag 無効化

Phase 1 と同様。

##### ステップ 2: ActorCell の復元

```bash
# MessageInvoker 分離前のActorCell実装に戻す
git revert -m 1 <phase-2b-merge-commit>

# ActorCell が直接メッセージ処理を行う従来の実装を確認
grep -r "invoke_message" modules/actor-core/src/api/actor_cell.rs
```

##### ステップ 3: ミドルウェアチェインの無効化

```rust
// middleware を無効化し、直接実行に戻す
#[cfg(not(feature = "new-scheduler"))]
fn process_message(&self, msg: Envelope) -> Result<(), ActorError> {
  // ミドルウェアを経由せず直接実行
  self.actor.receive(msg)
}
```

#### 検証ポイント

- [ ] ActorCell が直接メッセージ処理を行っている
- [ ] ミドルウェアチェインが無効化されている
- [ ] Guardian との連携が正常に動作している

---

### Phase 3: ランタイム抽象化とno_std対応

**対象ブランチ**: `refactor/runtime-abstraction`
**Feature Flag**: `new-scheduler` + `runtime-tokio` / `runtime-embassy`

#### ロールバック手順

##### ステップ 1: Runtime Feature Flag の無効化

```bash
# Tokio ランタイムに固定
cargo build --features std,runtime-tokio --no-default-features
```

##### ステップ 2: Tokio 固有実装への復帰

```rust
// ランタイム抽象化前の Tokio 直接使用コードに戻す
#[cfg(not(feature = "new-scheduler"))]
use tokio::spawn;

#[cfg(feature = "new-scheduler")]
use runtime_abstraction::spawn;
```

##### ステップ 3: no_std ビルドの無効化

```bash
# no_std ビルドを一時的に無効化
# CI で no_std ターゲットをスキップ
sed -i 's/--target thumbv7em-none-eabihf/# --target thumbv7em-none-eabihf/' .github/workflows/ci.yml
```

#### 組み込み環境での対応

- **Embassy 環境**: Tokio 環境へのフォールバック手順を実行
- **ベアメタル**: 従来の組み込み実装（Phase 2 以前）に戻す

---

### Phase 4: 最適化と最終統合

**対象ブランチ**: `refactor/final-optimization`
**Feature Flag**: `new-scheduler` (デフォルト有効化検討)

#### ロールバック手順

##### ステップ 1: パフォーマンス最適化の巻き戻し

```bash
# lock-free 最適化など Phase 4 固有の変更を revert
git revert -m 1 <phase-4-merge-commit>
```

##### ステップ 2: Feature Flag のデフォルト設定変更

```toml
# Phase 4 でデフォルト有効化されていた場合、無効に戻す
[features]
default = ["std"]  # new-scheduler を除外
```

##### ステップ 3: 旧実装の完全復元（最終手段）

```bash
# Phase 0 以前の実装タグにロールバック
git tag phase-0-baseline $(git log --grep="Phase 0 start" --format="%H" -1)
git reset --hard phase-0-baseline
git push --force origin main
```

**警告**: force push は最終手段であり、チーム全体への通知が必須。

---

## 3. ロールバック後の対応

### 3.1 ポストモーテム実施

ロールバック後 48時間以内にポストモーテムミーティングを実施：

**議題**:
- 何が問題だったか（根本原因分析）
- なぜ検出が遅れたか（テスト・モニタリングの gap）
- 再発防止策（設計変更、テスト追加、フラグ戦略見直し）

**ドキュメント化**:
- `docs/postmortem/YYYY-MM-DD-phase-X-rollback.md` に記録
- ADR で設計判断の修正を文書化

### 3.2 ベンチマークと回帰テスト

```bash
# ロールバック後のベンチマーク実行
cargo bench --bench mailbox_throughput --features std
cargo bench --bench scheduler_latency --features std

# ベースラインと比較
python3 scripts/compare_benchmarks.py \
  --baseline benchmarks/baseline_before_refactor.md \
  --current target/criterion/
```

### 3.3 再試行計画の策定

- **即座の再試行は禁止**: 最低 1週間の分析期間を設ける
- **改善案の文書化**: ADR で修正方針を明記
- **段階的ロールアウト**: Feature flag + カナリアデプロイで再導入

---

## 4. 緊急連絡先

| 役割 | 担当者 | 連絡手段 |
|------|--------|----------|
| テックリード | TBD | Slack DM / PagerDuty |
| SRE オンコール | TBD | PagerDuty |
| プロジェクトマネージャー | TBD | Slack / Email |
| アーキテクト | TBD | Slack / Email |

## 5. ロールバック判断フローチャート

```
問題発生
    ↓
Critical か？ ── Yes → 即座にFeature Flag無効化 → オンコールに通知
    ↓ No
パフォーマンス劣化 >10% か？ ── Yes → 1時間以内にGit revert → テックリードに通知
    ↓ No
パフォーマンス劣化 5-10% か？ ── Yes → 24時間以内に判断 → PMと協議
    ↓ No
マイナーな問題 → 次回リリースで修正 → Issue登録
```

## 6. 付録

### A. ロールバックコマンド クイックリファレンス

```bash
# Feature flag 無効化
sed -i 's/default = \["std", "new-scheduler"\]/default = ["std"]/' Cargo.toml

# Git revert
git revert -m 1 <merge-commit-hash>

# デプロイ
./scripts/deploy.sh production --version=rollback

# ベンチマーク実行
cargo bench --features std

# メトリクス確認
kubectl top pods -l app=actor-system
```

### B. モニタリングダッシュボード

- **Grafana**: `https://grafana.example.com/d/actor-scheduler`
- **Prometheus**: `https://prometheus.example.com`
- **Logs**: `kubectl logs -f deployment/actor-system`

### C. 関連ドキュメント

- [ActorScheduler Refactor Design](../design/actor_scheduler_refactor.md)
- [Phase 1 Implementation Guide](../implementation/phase1_implementation_guide.md)
- [ADR Template](../adr/template.md)
- [Benchmark Comparison Script](../../scripts/compare_benchmarks.py)

---

**免責事項**: この手順書は Phase 0 時点でのドラフトです。各フェーズの実装完了時に実際の構成に合わせて更新してください。

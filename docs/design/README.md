# Actor Core 設計ドキュメント

このディレクトリには `cellex-actor-core-rs` の設計ドキュメントが含まれています。

## ドキュメント一覧

### アーキテクチャ
- **[actor-core-module-architecture.md](./actor-core-module-architecture.md)** - 現在のモジュール構成の全体像
  - 3層アーキテクチャ(api/internal/shared)
  - 各モジュールの責務と設計課題
  - 155ファイル・23モジュールの詳細分析

- **[ideal-module-structure.md](./ideal-module-structure.md)** - 理想的なモジュール構成
  - 137ファイル(-18, -11.6%)への削減計画
  - モジュールごとのBefore/After比較
  - 具体的な改善提案

- **[module-dependency-analysis.md](archive/module-dependency-analysis.md)** - 依存関係分析
  - 循環依存の検出(3箇所)
  - 設計原則違反の特定(shared→api依存)
  - 依存関係改善のロードマップ

### 移行ガイド
- **[migration-guide.md](./migration-guide.md)** - モジュール再編成の具体的手順
  - 5つのフェーズに分けた段階的移行計画
  - 具体的なコマンド付き実装手順
  - 後方互換性を維持する戦略
  - 12ヶ月のDeprecationスケジュール

### 技術的設計
- **[synchronization-abstraction.md](./synchronization-abstraction.md)** - 同期プリミティブの抽象化
  - `spin::Mutex` vs `tokio::Mutex` vs `std::Mutex`
  - ランタイム別の最適化戦略
  - `RuntimeMutex<T>` 抽象化の設計と実装計画
- **[D21-dead-letter-next-actions.md](archive/D21-dead-letter-next-actions.md)** - DeadLetter / Process Registry 再設計
  - ProcessRegistry 再構築と PID 解決戦略
  - DeadLetter メッセージと Control チャネル保持
  - Remote/Cluster 統合テスト計画

## 設計の優先順位

### Critical (即座に対処)
1. **同期プリミティブ抽象化** (synchronization-abstraction.md)
   - Tokio環境での効率化
   - 組み込み環境との互換性維持

2. **循環依存の解消** (module-dependency-analysis.md)
   - `api/actor` ⇄ `internal/context`
   - `shared` → `api` 依存

### High (優先的に対処)
3. **Scheduler簡略化** (migration-guide.md Phase 2)
   - 14サブモジュール → 3サブディレクトリ
   - Noop系の統合

4. **メッセージング統合** (ideal-module-structure.md)
   - 9サブモジュール → 6ファイル
   - metadata関連の整理

### Medium (計画的に対処)
5. **Supervision統合** (migration-guide.md Phase 4)
   - failure処理の統合
   - escalation機構の整理

## 推奨読み順

### 1. 初めて読む場合
1. **README.md** (このファイル) - 全体像の把握
2. **actor-core-module-architecture.md** - 現状理解
3. **ideal-module-structure.md** - 目指すべき姿
4. **migration-guide.md** - 具体的な実装計画

### 2. 特定の問題に取り組む場合
- **モジュール構成を改善したい** → ideal-module-structure.md
- **循環依存を解決したい** → module-dependency-analysis.md
- **Mutex抽象化を実装したい** → synchronization-abstraction.md
- **実際に移行作業をする** → migration-guide.md

### 3. 新機能を追加する場合
1. **ideal-module-structure.md** - どこに配置すべきか確認
2. **module-dependency-analysis.md** - 依存関係ルールの確認
3. **actor-core-module-architecture.md** - 既存パターンの参照

## 主要な設計原則

### 1. レイヤー分離
```
api → internal → shared
 ↓       ↓         ↓
外部利用 ← lib.rs (re-exports) ← crate root
```

**許可される依存**:
- ✅ `api` → `internal` (公開APIが内部実装を使用)
- ✅ `internal` → `shared` (内部実装が共有ユーティリティを使用)
- ✅ `api` → `shared` (公開APIが共有型を使用)

**禁止される依存**:
- ❌ `internal` → `api` (循環依存の防止)
- ❌ `shared` → `api` or `internal` (共有層は最下層)

### 2. モジュール構成
- **Rust 2018エディション**: `mod.rs`は使用禁止
- モジュールと同名のファイルを使用(例: `foo.rs` と `foo/` ディレクトリ)
- サブモジュールは親モジュール名のディレクトリ内に配置

### 3. テスト配置
- **単体テスト**: 実装の横に配置(例: `actor_context.rs` の隣に `actor_context/tests.rs`)
- **結合テスト**: `tests/` ディレクトリにのみ配置
- **`*_test.rs`ファイルは使用しない**

### 4. 公開API
- `api/*` モジュール → `pub` で公開、`lib.rs` で re-export
- `internal/*` モジュール → 原則非公開、一部の型のみ `lib.rs` で公開
- `shared/*` モジュール → 必要に応じて `lib.rs` で公開

## 設計決定の記録

### ADR-001: spin::Mutex から RuntimeMutex への移行
- **状態**: 提案
- **決定**: Feature flagベースの条件コンパイルを採用
- **理由**: シンプル、ゼロコスト、後方互換性
- **詳細**: [synchronization-abstraction.md](./synchronization-abstraction.md)

### ADR-002: Scheduler モジュールの簡略化
- **状態**: 計画中
- **決定**: 14サブモジュールを3サブディレクトリ(core/ready_queue/timeout)に統合
- **理由**: 複雑性の削減、保守性の向上
- **詳細**: [migration-guide.md Phase 2](./migration-guide.md#phase-2-scheduler-簡略化優先度-high)

### ADR-003: 循環依存の解消
- **状態**: Critical
- **決定**: trait-based抽象化による依存方向の逆転
- **理由**: レイヤー分離の徹底、保守性の向上
- **詳細**: [module-dependency-analysis.md](archive/module-dependency-analysis.md#循環依存の検出)

## 貢献ガイドライン

### 新しい設計ドキュメントを追加する場合
1. `docs/design/` に markdown ファイルを作成
2. このREADME.mdの「ドキュメント一覧」に追加
3. 関連する既存ドキュメントへのリンクを追加

### 既存ドキュメントを更新する場合
1. 変更理由を commit message に明記
2. 影響を受ける他のドキュメントも更新
3. 「設計決定の記録」セクションに追記(必要に応じて)

### レビュー基準
- [ ] 設計原則に沿っているか
- [ ] 後方互換性が維持されているか
- [ ] パフォーマンスへの影響が考慮されているか
- [ ] 移行パスが明確か

## 関連ドキュメント

### プロジェクトルート
- [CLAUDE.md](../../CLAUDE.md) - プロジェクト全体のガイドライン
- [PROJECT_STATUS.md](../../PROJECT_STATUS.md) - プロジェクト状況
- [Cargo.toml](../../Cargo.toml) - ワークスペース設定

### その他の設計ドキュメント
- `docs/architecture/` - システムアーキテクチャ(今後作成予定)
- `docs/api/` - API仕様(今後作成予定)
- `docs/performance/` - パフォーマンス分析(今後作成予定)

## バージョン管理

このディレクトリのドキュメントは以下のバージョン管理ルールに従います:

- **Major変更**: アーキテクチャの大幅な変更(例: 3層→4層)
- **Minor変更**: モジュールの追加・削除・統合
- **Patch変更**: 誤字修正、説明の追加・明確化

現在のバージョン: **1.0.0** (2025年1月時点の設計)

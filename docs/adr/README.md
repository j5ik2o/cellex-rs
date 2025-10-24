# Architecture Decision Records (ADR)

このディレクトリには、cellex-rs プロジェクトにおける重要なアーキテクチャ上の決定を記録したドキュメント（ADR）が格納されています。

## ADR とは

Architecture Decision Record (ADR) は、ソフトウェアアーキテクチャにおける重要な決定とその理由を記録するためのドキュメントです。

### ADR の目的

- **意思決定の透明性**: なぜその決定をしたのかを明確に記録
- **知識の共有**: チームメンバー間で設計思想を共有
- **将来の参照**: 過去の決定を振り返り、検証可能にする
- **議論の促進**: 代替案との比較を通じて、より良い決定を導く

## ADR の作成方法

### 1. テンプレートをコピー

```bash
cp docs/adr/template.md docs/adr/YYYY-MM-DD-brief-title.md
```

### 2. 命名規則

ADR ファイル名は以下の形式で命名します：

```
YYYY-MM-DD-brief-title.md
```

- `YYYY-MM-DD`: 作成日（例: 2025-10-22）
- `brief-title`: 簡潔なタイトル（ケバブケース、例: naming-policy）

### 3. 記入

テンプレートに従って、以下のセクションを埋めます：

- **ステータス**: 提案中 → 承認済み → (必要に応じて) 非推奨/置き換え済み
- **コンテキスト**: 背景、問題点、制約条件
- **決定**: 採用する解決策と代替案
- **結果**: 利点、欠点、影響、移行計画

### 4. レビュー

- Pull Request を作成
- 関連するチームメンバーにレビューを依頼
- 承認後、ステータスを「承認済み」に更新

## ADR の更新

ADR は一度作成したら**変更しない**のが原則です。

- 新しい決定が古い決定を置き換える場合は、**新しい ADR を作成**し、古い ADR のステータスを「置き換え済み」に更新
- 軽微な誤字修正や補足情報の追加は更新履歴に記録

## ステータスの遷移

```
提案中 (Proposed)
  ↓
承認済み (Accepted)
  ↓
非推奨 (Deprecated) または 置き換え済み (Superseded)
```

## 既存の ADR 一覧

### Phase 0: 設計フェーズ

- [2025-10-Phase0-naming.md](2025-10-Phase0-naming.md) - コンポーネント命名ポリシー
- [2025-10-Phase0-suspend-resume.md](2025-10-Phase0-suspend-resume.md) - Suspend/Resume 責務配置

(随時追加)

## 参考資料

- [Michael Nygard の ADR](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions)
- [ADR GitHub Organization](https://adr.github.io/)
- [Joel Parker Henderson's ADR templates](https://github.com/joelparkerhenderson/architecture-decision-record)

---

**最終更新**: 2025-10-22

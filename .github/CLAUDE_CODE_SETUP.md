# Claude Code GitHub Action セットアップガイド

このリポジトリでは、Claude Code GitHub Actionを使用してPRレビュー、コード改善、自動化タスクを実行できます。

## セットアップ手順

### 1. ワークフローファイルの有効化

ワークフローファイルのサンプルが `.github/examples/claude-code.workflow.yml` として提供されています。
これを有効にするには、以下のコマンドを実行してください:

```bash
# ワークフローファイルをコピー
cp .github/examples/claude-code.workflow.yml .github/workflows/claude-code.yml

# コミット&プッシュ
git add .github/workflows/claude-code.yml
git commit -m "feat: enable Claude Code GitHub Action workflow"
git push
```

**注意**: ワークフローファイルのプッシュには、リポジトリへの `workflows` 権限が必要です。
権限がない場合は、リポジトリ管理者に依頼してください。

### 2. APIキーの設定

このワークフローを使用するには、以下のシークレットがリポジトリに設定されている必要があります:

#### 必須シークレット

- `ANTHROPIC_API_KEY`: Anthropic APIキー
  - [Anthropic Console](https://console.anthropic.com/)で取得できます
  - リポジトリの Settings > Secrets and variables > Actions で追加してください

## 使い方

### 1. PRコメントでClaude Codeを呼び出す

PRのコメントで `@claude` をメンションすることで、Claude Codeを起動できます:

```
@claude このPRをレビューして、改善点を教えてください
```

```
@claude テストを追加してください
```

```
@claude パフォーマンスを改善してください
```

### 2. PRレビューコメントでClaude Codeを呼び出す

PRレビューの特定の行に対するコメントでも `@claude` をメンションできます:

```
@claude この関数をリファクタリングしてください
```

```
@claude ここにドキュメントコメントを追加してください
```

## Claude Codeが従う指針

Claude Codeは、`CLAUDE.md`に記載されている以下のプロジェクト指針に従います:

- **応対言語**: 日本語で応対
- **テスト**: すべてのテストをパス
- **コーディング規約**: Rust 2018エディション規約に準拠
- **一貫性**: 既存の実装を参考に一貫性のあるコードを作成
- **参考実装**: protoactor-goの実装を参考にする

## ワークフローの詳細

- **トリガー**: `issue_comment` および `pull_request_review_comment` イベント
- **条件**: コメントに `@claude` が含まれる場合のみ実行
- **タイムアウト**: 30分
- **権限**: contents (write), pull-requests (write), issues (write), checks (write)

## トラブルシューティング

### Claude Codeが起動しない

1. コメントに `@claude` が含まれているか確認
2. `ANTHROPIC_API_KEY` シークレットが正しく設定されているか確認
3. ワークフローの実行ログを確認（Actions タブ）

### APIキーのエラー

`ANTHROPIC_API_KEY` が有効で、適切な権限があることを確認してください。

## 参考リンク

- [Claude Code 公式ドキュメント](https://docs.claude.com/en/docs/claude-code)
- [claude-code-action GitHub](https://github.com/anthropics/claude-code-action)
- [Anthropic Console](https://console.anthropic.com/)

## ローカルでのClaude Code利用

GitHub Actionだけでなく、ローカルでもClaude Codeを利用できます:

```bash
# Claude Codeのインストール
npm install -g @anthropic-ai/claude-code

# プロジェクトディレクトリで実行
claude-code

# GitHub Appのセットアップ
claude-code /install-github-app
```

詳細は[Claude Code CLI ドキュメント](https://docs.claude.com/en/docs/claude-code)を参照してください。

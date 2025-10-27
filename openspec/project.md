# Project Context

## Purpose

[../README.md](README.md)

- 組み込み(no_std)、PC(std)稼働可能なアクターライブラリ
- Rustらしいアクターライブラリ
- protoactor-go, pekkoをかなり参考にする
- コアロジックは`no_std`にこだわる

## Tech Stack
- `*-core`
  - no_std
- `*-std`:
  - std::*
  - tokio
- `*-embedded`
  - no_std
  - embassy

## Project Conventions

### Code Style
- 原則的にRustの標準スタイルに従う
- 構造体のnewメソッドはScalaのプライマリーコンストラクターと同様に基本的にnewメソッドを必ず呼び出すこと

### Architecture Patterns
- 典型的なアクターシステムの設計に倣う
- プラガブルな機構によって、組み込み(no_std)、PC(std)でも稼働可能な仕組みを提供する

### Testing Strategy
- 単体テストでは`std::*`に依存してOK
- TDD/BDDを重視する

### Git Workflow
- Github Flowとする

## Domain Context
- [protoactor-go](docs/sources/protoactor-go), pekkoをかなり参考にする

## Important Constraints
- `*-core`モジュールはno_std。ロジックの中心地。拡張ポイントを提供し`*-std`,`*-embedded`モジュールから利用できるようにすること
- no_std組み込みでも`alloc::sync::Arc`が使えない場合があるので、Arc抽象機構`ArcShared`を使うこと

## External Dependencies
[Document key external services, APIs, or systems]

#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}"

THUMB_TARGETS=("thumbv6m-none-eabi" "thumbv8m.main-none-eabi")
DEFAULT_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable}"
FMT_TOOLCHAIN="${FMT_TOOLCHAIN:-nightly}"

usage() {
  cat <<'EOF'
使い方: scripts/ci.sh [コマンド...]
  lint       : cargo +nightly fmt -- --check を実行します
  clippy     : cargo clippy --workspace --all-targets -- -D warnings を実行します
  no-std     : no_std 対応チェック (core/utils) を実行します
  std        : std フィーチャーでのテストを実行します
  embedded   : embedded 系 (utils / actor) のチェックとテストを実行します
  embassy    : embedded のエイリアスです (互換目的)
  test       : ワークスペース全体のテストを実行します
  all        : 上記すべてを順番に実行します (引数なし時と同じ)
複数指定で部分実行が可能です (例: scripts/ci.sh lint test)
EOF
}

log_step() {
  printf '==> %s\n' "$1"
}

run_cargo() {
  if [[ -n "${DEFAULT_TOOLCHAIN}" ]]; then
    cargo "+${DEFAULT_TOOLCHAIN}" "$@"
  else
    cargo "$@"
  fi
}

ensure_target_installed() {
  local target="$1"

  if rustup target list --installed --toolchain "${DEFAULT_TOOLCHAIN}" | grep -qx "${target}"; then
    return 0
  fi

  if [[ -n "${CI:-}" ]]; then
    echo "info: installing target ${target} for toolchain ${DEFAULT_TOOLCHAIN}" >&2
    if rustup target add --toolchain "${DEFAULT_TOOLCHAIN}" "${target}"; then
      return 0
    fi
    echo "エラー: ターゲット ${target} のインストールに失敗しました。" >&2
    return 1
  fi

  echo "警告: ターゲット ${target} が見つからなかったためクロスチェックをスキップします。" >&2
  return 2
}

run_lint() {
  log_step "cargo +${FMT_TOOLCHAIN} fmt -- --check"
  cargo "+${FMT_TOOLCHAIN}" fmt -- --check
}

run_clippy() {
  log_step "cargo +${DEFAULT_TOOLCHAIN} clippy --workspace --all-targets -- -D warnings"
  run_cargo clippy --workspace --all-targets -- -D warnings
}

run_no_std() {
  log_step "cargo +${DEFAULT_TOOLCHAIN} check -p cellex-utils-core-rs --no-default-features --features alloc"
  run_cargo check -p cellex-utils-core-rs --no-default-features --features alloc

  log_step "cargo +${DEFAULT_TOOLCHAIN} check -p cellex-actor-core-rs --no-default-features --features alloc"
  run_cargo check -p cellex-actor-core-rs --no-default-features --features alloc
}

run_std() {
  log_step "cargo +${DEFAULT_TOOLCHAIN} test -p cellex-utils-core-rs --features std"
  run_cargo test -p cellex-utils-core-rs --features std

  log_step "cargo +${DEFAULT_TOOLCHAIN} test -p cellex-actor-core-rs --no-default-features --features std,unwind-supervision"
  run_cargo test -p cellex-actor-core-rs --no-default-features --features std,unwind-supervision

  log_step "cargo +${DEFAULT_TOOLCHAIN} test -p cellex-utils-std-rs"
  run_cargo test -p cellex-utils-std-rs

  log_step "cargo +${DEFAULT_TOOLCHAIN} test -p cellex-actor-std-rs"
  run_cargo test -p cellex-actor-std-rs
}

run_embedded() {
  log_step "cargo +${DEFAULT_TOOLCHAIN} check -p cellex-utils-embedded-rs --no-default-features --features rc"
  run_cargo check -p cellex-utils-embedded-rs --no-default-features --features rc

  log_step "cargo +${DEFAULT_TOOLCHAIN} check -p cellex-utils-embedded-rs --no-default-features --features arc"
  run_cargo check -p cellex-utils-embedded-rs --no-default-features --features arc

  log_step "cargo +${DEFAULT_TOOLCHAIN} test -p cellex-utils-embedded-rs --no-default-features --features embassy --no-run"
  run_cargo test -p cellex-utils-embedded-rs --no-default-features --features embassy --no-run

  log_step "cargo +${DEFAULT_TOOLCHAIN} check -p cellex-actor-embedded-rs --no-default-features --features alloc,embedded_arc"
  run_cargo check -p cellex-actor-embedded-rs --no-default-features --features alloc,embedded_arc

  log_step "cargo +${DEFAULT_TOOLCHAIN} test -p cellex-actor-embedded-rs --no-default-features --features alloc,embedded_arc"
  run_cargo test -p cellex-actor-embedded-rs --no-default-features --features alloc,embedded_arc

  for target in "${THUMB_TARGETS[@]}"; do
    if ! ensure_target_installed "${target}"; then
      status=$?
      if [[ ${status} -eq 1 ]]; then
        return 1
      fi
      continue
    fi

    log_step "cargo +${DEFAULT_TOOLCHAIN} check -p cellex-actor-core-rs --target ${target} --no-default-features --features alloc"
    run_cargo check -p cellex-actor-core-rs --target "${target}" --no-default-features --features alloc

    log_step "cargo +${DEFAULT_TOOLCHAIN} check -p cellex-actor-embedded-rs --target ${target} --no-default-features --features alloc,embedded_rc"
    run_cargo check -p cellex-actor-embedded-rs --target "${target}" --no-default-features --features alloc,embedded_rc
  done
}

run_tests() {
  log_step "cargo +${DEFAULT_TOOLCHAIN} test --workspace --verbose"
  run_cargo test --workspace --verbose
}

run_all() {
  run_lint
  run_no_std
  run_std
  run_embedded
  run_tests
}

main() {
  "${SCRIPT_DIR}/check_modrs.sh"

  if [[ $# -eq 0 ]]; then
    run_all
    return
  fi

  local status=0
  for cmd in "$@"; do
    case "${cmd}" in
      lint)
        run_lint
        ;;
      clippy)
        run_clippy
        ;;
      no-std|nostd)
        run_no_std
        ;;
      std)
        run_std
        ;;
      embedded|embassy)
        run_embedded
        ;;
      test|tests|workspace)
        run_tests
        ;;
      all)
        run_all
        ;;
      *)
        usage
        return 1
        ;;
    esac
  done
  return "${status}"
}

main "$@"

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
使い方: scripts/ci-check.sh [コマンド...]
  lint                   : cargo +nightly fmt -- --check を実行します
  dylint [lint...]       : カスタムリントを実行します (デフォルトはすべて、例: dylint mod-file-lint)
                           CSV 形式のショートハンドも利用可能です (例: dylint:mod-file-lint,module-wiring-lint)
  clippy                 : cargo clippy --workspace --all-targets -- -D warnings を実行します
  no-std                 : no_std 対応チェック (core/utils) を実行します
  std                    : std フィーチャーでのテストを実行します
  embedded / embassy     : embedded 系 (utils / actor) のチェックとテストを実行します
  test                   : ワークスペース全体のテストを実行します
  all                    : 上記すべてを順番に実行します (引数なし時と同じ)
複数指定で部分実行が可能です (例: scripts/ci-check.sh lint dylint module-wiring-lint)
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
  cargo "+${FMT_TOOLCHAIN}" fmt --all -- --check
}

run_dylint() {
  local -a requested=("$@")

  local -a lint_entries=(
    "mod-file-lint:lints/mod-file-lint"
    "module-wiring-lint:lints/module-wiring-lint"
    "tests-location-lint:lints/tests-location-lint"
  )

  local -a selected=()

  if [[ ${#requested[@]} -eq 0 ]]; then
    selected=("${lint_entries[@]}")
  else
    local name
    for name in "${requested[@]}"; do
      local found=""
      local entry
      for entry in "${lint_entries[@]}"; do
        local crate="${entry%%:*}"
        local alias="${crate//-/_}"
        if [[ "${name}" == "${crate}" || "${name}" == "${alias}" || "${name}" == "lints/${crate}" || "${name}" == "${entry#*:}" ]]; then
          local already=""
          if [[ ${#selected[@]} -gt 0 ]]; then
            local existing
            for existing in "${selected[@]}"; do
              if [[ "${existing}" == "${entry}" ]]; then
                already="yes"
                break
              fi
            done
          fi
          if [[ -z "${already}" ]]; then
            selected+=("${entry}")
          fi
          found="yes"
          break
        fi
      done
      if [[ -z "${found}" ]]; then
        echo "エラー: 未知のリンター '${name}' が指定されました" >&2
        echo "利用可能: ${lint_entries[*]//:*/}" >&2
        return 1
      fi
    done
  fi

  local toolchain
  toolchain="nightly-$(rustc +nightly -vV | awk '/^host:/{print $2}')"
  local -a lib_dirs=()
  local -a dylint_args=()

  local entry
  for entry in "${selected[@]}"; do
    local crate="${entry%%:*}"
    local lint_path="${entry#*:}"
    local lib_name="${crate//-/_}"

    log_step "cargo +nightly build --manifest-path ${lint_path}/Cargo.toml --release"
    CARGO_NET_OFFLINE="${CARGO_NET_OFFLINE:-true}" cargo +nightly build --manifest-path "${lint_path}/Cargo.toml" --release

    log_step "cargo +nightly test --manifest-path ${lint_path}/Cargo.toml -- test ui -- --quiet"
    CARGO_NET_OFFLINE="${CARGO_NET_OFFLINE:-true}" cargo +nightly test --manifest-path "${lint_path}/Cargo.toml" -- test ui -- --quiet

    local target_dir="${lint_path}/target/release"
    local plain_lib="${target_dir}/lib${lib_name}.dylib"
    local tagged_lib="${target_dir}/lib${lib_name}@${toolchain}.dylib"

    if [[ -f "${plain_lib}" ]]; then
      cp -f "${plain_lib}" "${tagged_lib}"
      lib_dirs+=("$(cd "${target_dir}" && pwd)")
    else
      echo "エラー: ${plain_lib} が見つかりません" >&2
      return 1
    fi

    dylint_args+=("--lib" "${lib_name}")
  done

  local dylint_library_path
  dylint_library_path="$(IFS=:; echo "${lib_dirs[*]}")"

  log_step "cargo +${DEFAULT_TOOLCHAIN} dylint --workspace ${dylint_args[*]} --no-build --no-metadata"
  DYLINT_LIBRARY_PATH="${dylint_library_path}" CARGO_NET_OFFLINE="${CARGO_NET_OFFLINE:-true}" run_cargo dylint --workspace "${dylint_args[@]}" --no-build --no-metadata
}

run_clippy() {
  log_step "cargo +${DEFAULT_TOOLCHAIN} clippy --workspace --exclude module-wiring-lint --all-targets -- -D warnings"
  run_cargo clippy --workspace --exclude module-wiring-lint --all-targets -- -D warnings
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

  log_step "cargo +${DEFAULT_TOOLCHAIN} test -p cellex-remote-core-rs"
  run_cargo test -p cellex-remote-core-rs

  log_step "cargo +${DEFAULT_TOOLCHAIN} test -p cellex-cluster-core-rs"
  run_cargo test -p cellex-cluster-core-rs
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
  log_step "cargo +${DEFAULT_TOOLCHAIN} test --workspace --exclude module-wiring-lint --verbose"
  run_cargo test --workspace --exclude module-wiring-lint --verbose
}

run_all() {
  run_lint
  run_dylint
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
  while [[ $# -gt 0 ]]; do
    case "$1" in
      lint)
        run_lint || status=$?
        shift
        ;;
      dylint)
        shift
        local -a lint_args=()
        while [[ $# -gt 0 ]]; do
          case "$1" in
            lint|dylint|dylint:*|clippy|no-std|nostd|std|embedded|embassy|test|tests|workspace|all)
              break
              ;;
            --)
              shift
              break
              ;;
            *)
              lint_args+=("$1")
              shift
              ;;
          esac
        done
        if [[ ${#lint_args[@]} -eq 0 ]]; then
          if ! run_dylint; then
            return 1
          fi
        else
          if ! run_dylint "${lint_args[@]}"; then
            return 1
          fi
        fi
        ;;
      dylint:*)
        local spec="${1#dylint:}"
        local -a lint_args=()
        IFS=',' read -r -a lint_args <<< "${spec}"
        if [[ ${#lint_args[@]} -eq 0 || ( ${#lint_args[@]} -eq 1 && -z "${lint_args[0]}" ) ]]; then
          if ! run_dylint; then
            return 1
          fi
        else
          if ! run_dylint "${lint_args[@]}"; then
            return 1
          fi
        fi
        shift
        ;;
      clippy)
        run_clippy || status=$?
        shift
        ;;
      no-std|nostd)
        run_no_std || status=$?
        shift
        ;;
      std)
        run_std || status=$?
        shift
        ;;
      embedded|embassy)
        run_embedded || status=$?
        shift
        ;;
      test|tests|workspace)
        run_tests || status=$?
        shift
        ;;
      all)
        run_all || status=$?
        shift
        ;;
      --help|-h|help)
        usage
        return 0
        ;;
      --)
        shift
        if ! run_all; then
          return 1
        fi
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

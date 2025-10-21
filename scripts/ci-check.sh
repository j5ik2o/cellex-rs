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
  local -a cmd
  if [[ -n "${DEFAULT_TOOLCHAIN}" ]]; then
    cmd=(cargo "+${DEFAULT_TOOLCHAIN}" "$@")
  else
    cmd=(cargo "$@")
  fi

  if ! "${cmd[@]}"; then
    echo "error: ${cmd[*]}" >&2
    return 1
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
  local -a lint_filters
  lint_filters=()
  local -a module_filters
  module_filters=()

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -n|--name)
        if [[ $# -lt 2 ]]; then
          echo "エラー: -n/--name にはリント名を指定してください" >&2
          return 1
        fi
        lint_filters+=("$2")
        shift 2
        ;;
      -m|--module|--package)
        if [[ $# -lt 2 ]]; then
          echo "エラー: -m/--module にはモジュール名(またはパッケージ名)を指定してください" >&2
          return 1
        fi
        module_filters+=("$2")
        shift 2
        ;;
      --)
        shift
        while [[ $# -gt 0 ]]; do
          lint_filters+=("$1")
          shift
        done
        ;;
      -h|--help)
        echo "利用例: scripts/ci-check.sh dylint -n mod-file-lint -m cellex-actor-core-rs" >&2
        return 0
        ;;
      *)
        lint_filters+=("$1")
        shift
        ;;
    esac
  done

  local -a requested=()
  if [[ ${lint_filters+set} == set ]]; then
    requested=("${lint_filters[@]}")
  fi

  local -a lint_entries=(
    "mod-file-lint:lints/mod-file-lint"
    "module-wiring-lint:lints/module-wiring-lint"
    "type-per-file-lint:lints/type-per-file-lint"
    "tests-location-lint:lints/tests-location-lint"
    "use-placement-lint:lints/use-placement-lint"
    "rustdoc-lint:lints/rustdoc-lint"
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

  local -a package_args=()
  if [[ ${#module_filters[@]} -gt 0 ]]; then
    local module_spec
    for module_spec in "${module_filters[@]}"; do
      local manifest_path=""
      local package_name=""

      if [[ -f "${module_spec}" && "${module_spec}" == *Cargo.toml ]]; then
        manifest_path="${module_spec}"
      elif [[ -d "${module_spec}" && -f "${module_spec}/Cargo.toml" ]]; then
        manifest_path="${module_spec}/Cargo.toml"
      elif [[ -d "modules/${module_spec}" && -f "modules/${module_spec}/Cargo.toml" ]]; then
        manifest_path="modules/${module_spec}/Cargo.toml"
      fi

      if [[ -n "${manifest_path}" ]]; then
        package_name="$(awk -F'"' '/^name[[:space:]]*=/{print $2; exit}' "${manifest_path}")"
      else
        package_name="${module_spec}"
      fi

      if [[ -z "${package_name}" ]]; then
        echo "エラー: モジュール '${module_spec}' のパッケージ名を特定できませんでした" >&2
        return 1
      fi

      local already=""
      if [[ ${#package_args[@]} -gt 0 ]]; then
        local idx=0
        while [[ ${idx} -lt ${#package_args[@]} ]]; do
          if [[ "${package_args[${idx}+1]}" == "${package_name}" ]]; then
            already="yes"
            break
          fi
          idx=$((idx + 2))
        done
      fi

      if [[ -z "${already}" ]]; then
        package_args+=("-p" "${package_name}")
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

  local rustflags_value
  if [[ -n "${RUSTFLAGS-}" ]]; then
    rustflags_value="${RUSTFLAGS} -Dwarnings"
  else
    rustflags_value="-Dwarnings"
  fi

  local -a cargo_dylint_args=()
  if [[ ${#package_args[@]} -eq 0 ]]; then
    cargo_dylint_args+=("--workspace")
  else
    cargo_dylint_args+=("${package_args[@]}")
  fi
  cargo_dylint_args+=("${dylint_args[@]}")
  cargo_dylint_args+=("--no-build" "--no-metadata")

  log_step "cargo +${DEFAULT_TOOLCHAIN} dylint ${cargo_dylint_args[*]} (RUSTFLAGS=${rustflags_value})"
  RUSTFLAGS="${rustflags_value}" DYLINT_LIBRARY_PATH="${dylint_library_path}" CARGO_NET_OFFLINE="${CARGO_NET_OFFLINE:-true}" run_cargo dylint "${cargo_dylint_args[@]}"
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

  while [[ $# -gt 0 ]]; do
    case "$1" in
      lint)
        run_lint || return 1
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
        run_clippy || return 1
        shift
        ;;
      no-std|nostd)
        run_no_std || return 1
        shift
        ;;
      std)
        run_std || return 1
        shift
        ;;
      embedded|embassy)
        run_embedded || return 1
        shift
        ;;
      test|tests|workspace)
        run_tests || return 1
        shift
        ;;
      all)
        run_all || return 1
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
  return 0
}

main "$@"

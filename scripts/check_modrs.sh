#!/usr/bin/env bash
# scripts/check_modrs.sh

set -euo pipefail

results=$(find . \
  -path "./docs/sources" -prune -o \
  -path "./lints/mod-file-lint/tests/ui/auxiliary" -prune -o \
  -type f -name "mod.rs" -print)

if [[ -n "${results}" ]]; then
  echo "Error: mod.rs files are not allowed in this project."
  echo "The following files violate the rule:"
  echo "${results}"
  exit 1
fi

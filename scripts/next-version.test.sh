#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

# check <expected> <package version> [existing tags...]
check() {
  local want=$1 base=$2
  shift 2
  local got
  got=$(printf '%s\n' "$@" | ./next-version.sh "$base")
  [ "$got" = "$want" ] || { echo "FAIL: package $base, tags [$*]: wanted $want, got $got"; exit 1; }
}

check 0.1.0 0.1.0
check 0.1.1 0.1.0 v0.1.0
check 0.1.11 0.1.0 v0.1.9 v0.1.10
check 0.2.0 0.2.0 v0.1.3
check 0.2.1 0.2.0 v0.2.0
check 0.3.3 0.1.0 v0.1.0 v0.3.2
check 0.1.1 0.1.0 v0.1.0 latest v1 v0.1.0-rc.1 nightly

echo "next-version: ok"

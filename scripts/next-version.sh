#!/usr/bin/env bash
# Prints the version for the next release.
#
#   git tags, one per line, on stdin | next-version.sh <package.json version>
#
# That is the patch after the newest vX.Y.Z tag, unless the package version is already ahead of it
# (a minor or major bump in package.json wins). Other tags are ignored.
set -euo pipefail

base=$1
latest=$( (grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' || true) | sed 's/^v//' | sort -V | tail -1)

if [ -z "$latest" ]; then
  echo "$base"
  exit 0
fi

bumped=$(awk -F. '{ printf "%d.%d.%d", $1, $2, $3 + 1 }' <<< "$latest")
printf '%s\n%s\n' "$base" "$bumped" | sort -V | tail -1

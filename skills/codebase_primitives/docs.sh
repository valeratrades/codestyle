#!/usr/bin/env bash
# Docs shaping this project, most-shaping first. Usage: docs.sh [root]
set -euo pipefail
cd "${1:-.}"

[ -f docs/ARCHITECTURE.md ] && echo docs/ARCHITECTURE.md
[ -d docs/spec ] && find docs/spec -type f | sort

find . -name target -prune -o -name .git -prune -o -name Cargo.toml -print | while read -r manifest; do
	crate=$(dirname "$manifest")
	[ "$crate" = . ] && dirs=("$crate/src") || dirs=("$crate/src" "$crate/docs")
	find "${dirs[@]}" -name '.*' -prune -o \( -name '*.typ' -o -name '*.md' \) -print 2>/dev/null
done | sort -u

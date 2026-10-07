#!/bin/bash
# check-sanskriti-programs.sh — W-121
# Tests the extracted program logic (classOf and APP_LOAD) for the VSCode extension.

set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$DIR")"

# `mktemp`, not the fixed /tmp/test-programs.js: this is written and then RUN,
# so two concurrent gates could run each other's script (W-305).
js=$(mktemp "${TMPDIR:-/tmp}/test-programs.XXXXXX")
trap 'rm -f "$js"' EXIT
cat << 'EOF' > "$js"
const assert = require('assert');
const path = require('path');
const root = process.argv[2];
const { getAppLoad, classOf } = require(path.join(root, 'editor/sanskriti/src/programs.js'));

assert.strictEqual(getAppLoad(root), '०षोड्२०००००००');
assert.strictEqual(classOf(root, 'atithi.sas'), 'app');
assert.strictEqual(classOf(root, 'arena.sas'), 'boot-proof');
assert.strictEqual(classOf(root, 'does-not-exist.sas'), null);
console.log('Programs logic tests pass.');
EOF

node "$js" "$ROOT"

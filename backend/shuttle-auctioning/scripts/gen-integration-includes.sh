#!/usr/bin/env bash
# Regenerate tests/inc/*.rs — namespaced copies of src/{ledger,catalog}.rs
# spliced into the integration test crate (tests/integration_rp.rs), because
# the lib's modules are private. Each copy becomes `mod ledger { ... }` /
# `mod catalog { ... }` with crate-paths rewritten to self-paths and `//!`
# headers demoted to comments.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p tests/inc
for f in ledger catalog; do
  sed -e 's|^//!|    //|' \
      -e 's|crate::ledger|super::ledger|g' \
      -e 's|crate::catalog|super::catalog|g' \
      "src/$f.rs" > "tests/inc/$f.body"
  {
    echo "mod $f {"
    cat "tests/inc/$f.body"
    echo "}"
  } > "tests/inc/$f.rs"
  rm "tests/inc/$f.body"
done
echo "regenerated $(ls tests/inc/*.rs | tr '\n' ' ')"

prepare:
    @cargo run --package lunaris_plugin_registrar

build-release:
    @cargo build --package lunaris_core --release

cleanup:
    @cargo run --package lunaris_build_cleaner

run:
    -just prepare
    cargo run --package lunaris_core
    just cleanup

build-full:
    just prepare
    - just build-release
    just cleanup

build:
    -just cleanup
    -just prepare
    -just dbg
    -just cleanup

check:
    -just prepare
    cargo check --package lunaris_core

clippy:
    -just prepare
    cargo clippy --package lunaris_core

c:
    -just clippy

dbg:
    @cargo build --package lunaris_core

run:
    cargo run --package lunaris_core

rr:
    cargo run --package lunaris_core --release

build-full:
    cargo build --package lunaris_core --release

build:
    just dbg

check:
    cargo check --package lunaris_core

clippy:
    cargo clippy --package lunaris_core

c:
    -just clippy

dbg:
    @cargo build --package lunaris_core

HACK := "cargo hack --feature-powerset --exclude-features default,docs-features,std -p tracing-wide"

check: check-native check-wasm

check-native:
    {{HACK}} check

check-wasm:
    {{HACK}} check --target wasm32-unknown-unknown

docs:
    rm -rf target/doc
    RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps

examples: example-catalogue-facet example-catalogue-serde example-instrument example-subscriber example-subscriber-facet example-subscriber-serde example-tags

example-catalogue-facet:
    cargo run --example catalogue-facet --features catalogue,facet

example-catalogue-serde:
    cargo run --example catalogue-serde --features catalogue,serde

example-instrument:
    cargo run --example instrument --features instrument,subscriber

example-subscriber:
    cargo run --example subscriber --features subscriber

example-subscriber-facet:
    cargo run --example subscriber-facet --features facet,subscriber

example-subscriber-serde:
    cargo run --example subscriber-serde --features subscriber,serde

example-tags:
    cargo run --example tags --features subscriber

lint: fmt-check clippy

fmt-check:
    cargo fmt --all --check

clippy:
    cargo clippy --workspace --all-features --all-targets -- -D warnings

test: test-native test-wasm

test-native:
    {{HACK}} test --test coverage --test cross_crate
    # Targets the powerset can't reach, run once at --all-features (mirrors mise's
    # old `cargo test --workspace --all-features`): lib unit tests (the host-only
    # macro-internal tests), doctests (`--doc` is exclusive of `--test`), and the
    # `ui` trybuild suite (native-only, rustc-version-sensitive).
    cargo test --workspace --all-features --lib
    cargo test --workspace --all-features --doc
    cargo test -p tracing-wide --all-features --test ui

test-wasm:
    {{HACK}} test --test coverage --test cross_crate --target wasm32-unknown-unknown

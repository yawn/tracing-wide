HACK := "cargo hack --feature-powerset --exclude-features default,docs,std -p tracing-wide"
TESTS := "--test core --test subscriber --test catalogue --test instrument --test serde --test facet --test cross_crate"

check: check-native check-wasm

check-native:
    {{HACK}} check

check-wasm:
    {{HACK}} check --target wasm32-unknown-unknown

clippy:
    cargo clippy --workspace --all-features --all-targets -- -D warnings

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

test: test-examples test-native test-wasm

test-examples: # set TRYCMD=overwrite when examples have changed
    cargo test -p tracing-wide --test examples

test-native:
    cargo test -p tracing-wide --no-default-features {{TESTS}}
    cargo test -p tracing-wide --all-features {{TESTS}}
    cargo test --workspace --all-features --lib
    cargo test --workspace --all-features --doc
    cargo test -p tracing-wide --all-features --test ui

test-wasm:
    cargo test -p tracing-wide --no-default-features {{TESTS}} --target wasm32-unknown-unknown
    cargo test -p tracing-wide --all-features {{TESTS}} --target wasm32-unknown-unknown

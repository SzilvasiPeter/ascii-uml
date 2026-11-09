fmt:
    cargo fmt --all -- --check

clippy:
    cargo clippy --all-targets --all-features -- \
        -D warnings \
        -W clippy::all -W clippy::pedantic -W clippy::nursery -W clippy::cargo

test:
    cargo test --all-targets --all-features

coverage:
    cargo llvm-cov --all-targets --all-features --workspace --html

open:
    cargo llvm-cov --html --open

check: fmt clippy test

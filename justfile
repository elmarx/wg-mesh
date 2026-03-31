# https://just.systems

ci:
    cargo check --all-targets
    cargo fmt --check
    cargo clippy --all-targets -- -W clippy::pedantic
    # cargo nextest run

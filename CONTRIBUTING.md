# Contributing

## Development setup

Install Rust stable, then run:

```bash
cargo build
cargo test
cargo fmt --check
cargo clippy --workspace --all-targets
```

A helper script is available:

```bash
./scripts/dev-check.sh
```

## Project rules

- Do not add destructive file actions without tests and safety documentation.
- Do not copy source code from other duplicate-finder projects unless licensing is reviewed first.
- Keep scan logic in backend crates, not in the GUI.
- Every match engine must produce explainable match reasons.
- Every future action must be dry-runnable.

## Pull request checklist

Before opening a PR:

- code builds
- tests pass
- formatting passes
- new behavior is documented
- safety implications are described
- output format changes are noted in `CHANGELOG.md`

## Commit message style

Use simple scoped messages where practical:

```text
core: add file identity model
cli: add json output option
docs: document action safety model
gui: add source selection screen
```

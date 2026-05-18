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

On Windows with the GNU Rust toolchain, use:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\test-windows.ps1
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

## Ownership and review

- See `.github/CODEOWNERS` for the default review ownership map.
- Keep PRs focused when possible so scan, action, and GUI changes are easier to review safely.

## Versioning and release notes

- Keep user-visible behavior changes listed in `CHANGELOG.md`.
- Treat JSON, CSV, manifest, and action-plan shape changes as release-note-worthy.
- Call out safety-model changes explicitly in PR descriptions and release notes.

## Commit message style

Use simple scoped messages where practical:

```text
core: add file identity model
cli: add json output option
docs: document action safety model
gui: add source selection screen
```

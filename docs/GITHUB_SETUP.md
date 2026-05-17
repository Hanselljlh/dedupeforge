# GitHub setup

This repository is ready to push to GitHub after choosing the final remote URL.

## 1. Rename the folder if desired

```bash
mv dedupeforge-mvp dedupeforge
cd dedupeforge
```

## 2. Initialize Git

```bash
git init
git add .
git commit -m "repo: initial DedupeForge MVP"
```

## 3. Create an empty GitHub repository

Create a new empty repository named `dedupeforge`.

Do not initialize it with a README, license, or `.gitignore`, because those files already exist locally.

## 4. Add the remote

Replace `OWNER` with your GitHub username or organization.

```bash
git remote add origin git@github.com:OWNER/dedupeforge.git
git branch -M main
git push -u origin main
```

HTTPS alternative:

```bash
git remote add origin https://github.com/OWNER/dedupeforge.git
git branch -M main
git push -u origin main
```

## 5. Update repository metadata

After creating the GitHub repo, update the placeholder in `Cargo.toml`:

```toml
repository = "https://github.com/OWNER/dedupeforge"
```

Replace it with the real repository URL.

## 6. Enable GitHub features

Recommended settings:

- enable Issues
- enable Discussions later if the project gets users
- enable Dependabot alerts
- protect the `main` branch after CI passes
- require PRs before merging once more contributors are involved

## 7. First GitHub issues to create

Suggested first issues:

```text
core: add test fixtures for exact duplicate scans
core: add SQLite cache crate
core: add action-plan model
actions: add quarantine move and undo manifest
cli: add config file support
gui: choose desktop frontend stack
media: design perceptual image hash engine
```

## 8. Suggested initial repository description

```text
Safe duplicate-file investigation tool with fast hashing, protected folders, and an AllDup-style review workflow.
```

## 9. Suggested topics

```text
duplicate-files duplicate-finder dedupe rust cli file-management blake3 xxhash photos media-library nas
```

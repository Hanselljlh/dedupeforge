# ADR 0003: Support fast and cryptographic hash options

## Status

Accepted

## Context

Large file collections need fast scanning. Some users also want conservative verification.

## Decision

Support multiple hash algorithms:

- BLAKE3 for fast modern hashing
- XXH3 128-bit for very fast non-cryptographic hashing
- SHA-256 for conservative cryptographic hashing

Also support optional byte-by-byte verification.

## Consequences

Positive:

- users can choose speed or conservatism
- non-destructive scans can be fast
- destructive actions can require stronger verification later

Negative:

- UI must explain the differences
- action planner must know when extra verification is required

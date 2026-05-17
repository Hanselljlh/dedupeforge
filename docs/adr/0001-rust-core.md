# ADR 0001: Use Rust for the core engine

## Status

Accepted

## Context

DedupeForge needs to perform filesystem walking, hashing, grouping, and later media processing over large collections. These workloads are I/O-heavy and benefit from safe concurrency.

## Decision

Use Rust for the reusable backend.

## Consequences

Positive:

- strong performance
- memory safety
- good concurrency tools
- can power both CLI and GUI
- good packaging options for desktop apps

Negative:

- higher learning curve than scripting languages
- GUI development may require additional frontend stack
- some media libraries may need wrappers around external tools

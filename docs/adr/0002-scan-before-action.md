# ADR 0002: Separate scanning from actions

## Status

Accepted

## Context

Duplicate tools can damage files if scanning and deletion are too tightly coupled. DedupeForge should support careful review and reversible cleanup.

## Decision

Scanning produces reports. Actions consume reports or action plans. Scanning never modifies files.

## Consequences

Positive:

- safer workflows
- easier testing
- reports can be reviewed/exported
- GUI and CLI can share the same model

Negative:

- more code is required before cleanup actions exist
- users must perform an additional review/action step

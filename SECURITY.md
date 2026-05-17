# Security policy

## Reporting issues

If you find a security or data-loss issue, open a private security advisory if the repository supports it, or contact the maintainers directly.

Do not publicly post a working exploit for destructive file behavior before maintainers have time to respond.

## Data-loss class bugs

For this project, the following are treated as high severity:

- deleting all files in a duplicate group
- modifying protected/reference paths
- action manifest mismatch
- restoring files to the wrong path
- trusting stale cache data for destructive actions
- following symlinks unexpectedly during destructive actions

## Current MVP

The current MVP has no destructive actions. It only scans and reports.

# Security policy

## Reporting issues

If you find a security or data-loss issue, open a private security advisory if the repository supports it, or contact the maintainers directly.

Do not publicly post a working exploit for destructive file behavior before maintainers have time to respond.

## Data-loss class bugs

For this project, the following are treated as high severity:

- deleting all files in a duplicate group
- modifying protected or reference paths
- action manifest mismatch
- restoring files to the wrong path
- trusting stale cache data for destructive actions
- following symlinks unexpectedly during destructive actions

## Current action model

The current prototype includes reversible quarantine moves, manifest-based restore, and opt-in hard-link and symlink replacement.

That means bugs in planning, quarantine, restore, replacement, or protected-path handling should be treated as security-sensitive for this project.

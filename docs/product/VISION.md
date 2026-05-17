# Product vision

DedupeForge should become a single application for duplicate investigation, cleanup planning, and safe cleanup execution.

## Desired experience

The user should be able to:

1. select one or more folders, drives, or network shares
2. choose a scan profile
3. protect known-good archive folders
4. run a scan
5. inspect each group with clear match reasons
6. preview files before action
7. auto-select likely duplicates using transparent rules
8. move selected files to quarantine
9. undo the action if needed

## Target feel

The GUI should be closer to AllDup than to a minimal scanner.

It should have:

- a dense but understandable results table
- detailed filters
- grouped duplicate results
- a preview panel
- per-group keep/delete state
- persistent scan profiles
- exportable reports

## Product boundaries

DedupeForge is not intended to be:

- a photo manager
- a music library manager
- a backup system
- a media transcoder
- a forensic evidence tool

It can support those workflows, but it should remain focused on duplicate discovery and safe cleanup.

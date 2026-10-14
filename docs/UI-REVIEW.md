# UI review — October 14, 2026

Version 0.5.2 keeps phones as the default surface; desktop enhancements begin at 960 px.

## Changes

- Consistent page gutters, 20 px phone card padding and 24 px desktop card padding.
- Readable 14 px field labels, 16 px input text, and controls at least 44–48 px tall.
- Consistent settings, backup, receipt, journal and helper text spacing.
- Long item names wrap without squeezing status controls or edit targets.
- Single-column phone modal forms, visible save/cancel actions and background scroll locking.
- Narrow-screen garage totals stay readable rather than wrapping the final digit.
- Optional agent settings appear at the end and span the desktop settings grid.
- Accessible names for dialogs and restrained black/charcoal surfaces.

## Verification

Browser review used an isolated SQLite database with synthetic records. Reviewed registration/sign-in, garage overview, project details, part editor, journal, spending, and settings, including the combined backup section. Verified no horizontal page overflow at narrow and standard phone widths; measured approximately 49 px input height and 16 px input text. Reviewed desktop settings in light mode, then returned to dark mode. Receipt scan and optional native-only update/discovery screens share the same spacing rules and were checked in source; physical-device validation of Android keyboard/safe-area behavior remains outstanding.

App data and domain rules are unchanged. Builds are local; GitHub Actions remains disabled.

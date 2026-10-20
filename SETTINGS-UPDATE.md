# Settings, icon, and folder cleanup — 1.0.2

Updated on 2026-10-15 for the Windows x64 Rust version of Aero.

## Requested changes

- Removed the exterior black/navy icon background. The 512px PNG has alpha 0 at its corners; the Windows ICO was regenerated with the Tauri icon tool. Active asset paths: `src-tauri/icons/icon.png`, `src-tauri/icons/icon.ico`, and `src/renderer/public/icon.png`.
- Moved Password Manager into Privacy & shields. Its text is padded 20px from the row edge and its button sits to the right of the description, including at the 900px minimum window width.
- Moved Restore defaults into Privacy & shields.
- Put Import browser data directly below Performance in the settings navigation, with its content inside the same central frame. Performance also links to this section. Import contents now have padding.
- Added selectable clearing for history, download records, cookies, disk cache, website storage, and general autofill, plus Select all/deselect all. Time ranges are Today, last week (7 days), last month (30 days), last year (365 days), and All time.
- Routed the toolbar flame, clear-history button, and cleanup shortcut to the selector. Data clears only after the user presses Clear selected data. The native Windows completion callback must succeed before app history/download metadata is updated.
- Fixed an existing theme bug: settings now honor Light/Dark/System instead of always following the system CSS preference. Password-manager styles are included in the stylesheet scan.
- Kept unchecked types and records outside the selected interval. Bookmarks, downloaded files, ordinary Aero preferences, and encrypted-vault passwords are outside this browsing-data action. The vault still requires its existing Windows authentication controls.
- Added timestamps for download/recently-closed records and separated normal/private download metadata. Prevented history from being recreated solely by title/state changes after clearing, and prevented private pages from being written to the normal history list.

## Verification

The full `npm test` suite passed: 4 time-range/selection checks, 19 Rust unit tests, and 33 live native browser checks (56 total). The frontend typecheck and production build passed as part of that run. Tests use synthetic pages, credentials, and temporary profiles.

New live checks cover Password Manager/Restore defaults placement, padding and inline button alignment, minimum-width light-theme layout, the import settings frame, selection controls, all five presets, native time-limited clearing, old-history preservation, unchecked cookies/storage preservation, invalid requests, and isolation of private website data.

The icon PNG's transparency was checked programmatically. Settings screenshots were inspected from actual WebView2 rendering. Proof files are in:
`C:/Users/rezae/Documents/Codex/2026-10-08/in-x20/work/settings-update-20261015/verification`.

As before, debug teardown can emit invalid-window-handle diagnostics for queued native operations after close; the suite exits successfully. Real Windows Hello flows and physical multi-monitor behavior remain manual checks from the initial port. No new claim of leak-proof password storage or measured browser speed is made.

## Folder cleanup

Removed 31 obsolete paths, including the unused nested `src-tauri/target` build cache (6,026,108,390 logical bytes), old demo entry points, old executables/static portable assets, an outdated non-runnable test stub, the redundant nested Cargo lock, duplicate blank HTML, and unused template/platform icons. Current Vite/Cargo entry points, dependencies, active root build cache, and browser profiles were preserved.

Source/configuration/icon backup before editing:
`C:/Users/rezae/Documents/Codex/2026-10-08/in-x20/work/aero-rust-settings-backup-20261015-092205`.

Hash-verified cleanup backups and removal manifest:
`C:/Users/rezae/Documents/Codex/2026-10-08/in-x20/work/settings-update-20261015/cleanup-backup` and sibling `cleanup-manifest.json` / `cleanup-result.json`.

Generated object files in the unused nested cache are reproducible from the source/locks and were not copied wholesale. Any old top-level executables found there were copied before removal. All 31 planned cleanup entries were removed successfully. Normal Aero source was outside this cleanup.

## Icon edit provenance

Built-in imagegen edit mode was used with transparent_background=true. Its output was converted to Windows/PNG icon formats using the official Tauri CLI. The generated original remains under the Codex generated_images directory; the active project consumes the saved copies listed above.

Prompt used:

> Edit target: the supplied Aero browser icon. Remove the entire black/dark navy background and the dark rounded-square tile behind the colorful circular emblem. Output a clean tightly framed centered transparent PNG app icon, with real alpha transparency outside the duck/blue-turquoise wave/orange lion/central shield emblem. Preserve the exact recognizable existing emblem artwork, colors, composition, shield, duck and lion, details, proportions; no redesign, no new text, no additions. Keep intentional dark shading/details inside the emblem and shield. Only the exterior black/navy backdrop/tile and surrounding exterior background should become transparent. Keep all emblem edges sharp and anti-aliased. Transparent background required.

## Windows clearing API

Category flags and time-limited clearing use [Microsoft's ICoreWebView2Profile2 API](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2profile2). Bounds are inclusive at the start and exclusive at the end. Website storage follows WebView2's site-level timestamps; it does not promise individual-key history or secure erasure from disk/backups.

## Final portable release

- File: `portable/Aero-Rust-Portable-1.0.2-x64.exe`.
- Size: 17,200,128 bytes (16.40 MiB), using the installed WebView2 runtime.
- SHA-256: `E4A5D2EF3E84DC759701241ECB55AAC203550BC4E63DAFC84062EC7251F0A0EA`.
- Final executable launched with a synthetic local page and isolated test profile; native window title was `Aero Browser`; close exited with code 0.
- The Windows icon extracted from the executable has corner alpha 0. Interface PNG transparency and both light/dark rendering were checked.
- Verified the final executable embeds `main-BZRQBXjH.js` and `index-BgCQWjYr.css`, matching the latest interface/style build.
- The full suite passed after cleanup and theme correction: 4 selection/time checks, 19 Rust tests, 33 live native checks.

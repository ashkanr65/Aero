# Rust repair report — 2026-10-14

This is the initial repair report. See [SETTINGS-UPDATE.md](SETTINGS-UPDATE.md) for the 1.0.2 settings/icon update and completed obsolete-file cleanup.

## Starting point

The Rust folder contained a Tauri demo and a separate root console program. Its tabs did not implement a real browser backend, its icon was a template icon, and its CSV preview logged the loaded file contents. Running the old console executable did not launch Aero.

## Changes

- Reused the current normal Aero React interface, settings schema, styling, keyboard mappings, password interface, and original PNG/ICO icons. The CSS output matches the reference stylesheet.
- Created a Rust Cargo workspace and a native Tauri/WebView2 backend, connecting Aero's existing browser interface to real native operations. The product has a distinct installer identity and executable filename.
- Implemented native tabs, navigation/history, split movement/sizing, tab order, private windows, audio indicators/muting, window controls, titlebar dragging, position notifications, fullscreen, zoom, downloads, profile imports, suggestions, printing/saving/capture, reader/read-aloud, filtering, sleeping tabs, and settings persistence.
- Implemented GPU disabling at browser-process startup. Efficiency mode uses a low-memory target instead of suspending background WebView2 controls, preserving sound playback.
- Ported the existing AES-GCM/DPAPI/Windows Hello vault backend to pure Rust, with trusted caller checks, isolated submission observation and autofill, authorization revocation, safe writes, compatible encrypted backups, and no CSV contents logging.
- Fixed event/lock ordering issues around shutdown and vault locking. Native checks close through the application's actual window controls, including after minimizing/restoring. Clear-on-exit awaits the Windows profile-clearing operation.
- Added portable staging and synthetic Rust/native browser regression checks. Demo-only code/assets are excluded from the build and retained to avoid destructive cleanup.

## Verification

The full frontend build, 15 Rust unit tests, and 22 live native checks passed. The portable release was also launched with a synthetic local page and temporary profile, and its native window closed with exit code 0. Automated credential tests use synthetic credentials and temporary files, never the user's real vault.

The unit suite covers tab movement/order/collapse, secure origins, import parsing, filter rules, AES-GCM tampering rejection, credential updates, malformed vaults, cancellation/interrupted writes, failed replacement, real current-user DPAPI encryption/decryption, restart/backup restoration, and the backend restore protocol. Test-only authentication substitution exercises backend mutations; it does not prove a real Hello prompt succeeds.

Native tests launch the real debug browser with embedded frontend and synthetic HTTPS fixtures, exercising shared UI, tab sizes, panes, settings/window controls, website IPC rejection, Web Audio/muting, isolated autofill, hostile prototype changes, hidden fields, private UI/vault denial, explicit trusted login submission, scripted-submit rejection, minimization/locking, native site-data clearing, and clear-on-exit. The test fixture/authentication substitutions are disabled in release builds.

## Remaining limitations

- Real Windows Hello success, cancellation, failure and unavailability; Windows session locking/sleep; clipboard interaction; physical monitor/taskbar behavior; and long browsing sessions need manual verification on target hardware.
- WebView2 and Electron ship different Chromium integrations. Exact website behavior and performance equivalence have not been established. No speed, RAM, battery, or smoothness improvement is claimed without benchmarks.
- Download records currently update at start/completion; continuous byte-progress reporting differs from the reference Electron implementation. Imports cover bookmarks/history; importing other browsers' active sessions is not supported.
- Spellchecking uses WebView2/Windows. Aero applies its toggle to ordinary page inputs/editable content in an isolated world and restores site attributes when re-enabled. Closed shadow-root controls and custom editors can behave differently; WebView2 has no equivalent global Electron spellcheck setting.
- The vault uses compatible normal Aero storage. Avoid editing it from both apps concurrently. DPAPI backups need the original user's Windows keys; same-user malware remains a threat. This has not had an independent security audit.
- Production npm dependencies had zero audit advisories at inspection. The retained Tailwind 3 development toolchain reported seven development-only advisories (five high, two moderate); no disruptive stylesheet migration was made solely to remove those build-tool warnings.
- Legacy demo executables may remain alongside the repaired app. Only the named `Aero-Rust-Portable-1.0.1-x64.exe` is the new portable release.

## Sources consulted

- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
- [Tauri security](https://v2.tauri.app/security/)
- [WebView2 runtime distribution](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)
- [WebView2 profile clearing](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2profile2)
- [Windows DPAPI](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata)
- [Windows desktop authentication interop](https://learn.microsoft.com/en-us/windows/win32/api/userconsentverifierinterop/nn-userconsentverifierinterop-iuserconsentverifierinterop)

## Final release verification

- Portable build: `npm run package:portable` — passed.
- Full suite: `npm test` — passed (15 Rust tests; 22 native checks; frontend typecheck/build).
- Release launch/window-close check — passed, native title `Aero Browser`, exit code 0, isolated synthetic profile.
- Original PNG and ICO source files match normal Aero byte for byte; the icon extracted from the release executable was visually checked.
- Production JavaScript dependency audit — zero advisories.
- Native debug teardown emitted WebView2 invalid-window-handle diagnostics for queued work after closing; the suite and release both exited successfully. These diagnostics should be monitored in extended session testing.
- Normal Aero source changes remained untouched.
- Executable: `portable/Aero-Rust-Portable-1.0.1-x64.exe`.
- Executable size: 17,183,744 bytes (16.39 MiB), using the separately installed WebView2 runtime.
- Executable SHA-256: `578D3CDC1F9097E11B341D62F6F95B06B9E1BFC461775B5E6F60474703498DE9`.
- Build verified with Rust 1.98.1 on Windows x64.

Automatic approval review rejected removal of legacy demo source/assets because deletion could permanently remove needed files. Those files were retained, excluded from the build, and documented rather than deleted.

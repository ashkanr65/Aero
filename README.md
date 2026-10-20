# Aero Browser Rust

This version keeps Aero's React interface and replaces the Electron main process with a Rust/Tauri backend. Web pages run in native Microsoft WebView2 controls, including separate controls for split panes. This is a Windows x64 application.

## Storage cleanup — October 15, 2026

The parent `Aero` folder now contains only `Aero_Rust`. The Rust source, tests, build configuration, documentation, current 1.0.3 portable executable, and its `AeroData` profile are retained.

The superseded Electron project, downloaded dependencies, build outputs, Cargo build cache, and generated Tauri schemas were removed. The packaged app runs independently of these files. To develop or rebuild later, follow the build instructions below; `npm ci` restores JavaScript dependencies and Cargo regenerates its build files.

Verified recovery archives of the old source/Git history and old browser profile are outside this project at `C:\Users\rezae\Documents\Codex\2026-10-08\in-x20\work\aero-storage-cleanup-20261015`. They total about 22 MB and exclude the large build/dependency caches. Earlier repair reports describe the project before this cleanup.

## Run the finished app

Open `portable/Aero-Rust-Portable-1.0.3-x64.exe`. This is the current portable release.

The Microsoft Edge WebView2 Evergreen Runtime must be installed. It is present on this development computer. The portable executable does not bundle an entire browser engine. See [Microsoft's runtime distribution guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution).

The portable app stores browser settings and WebView2 profiles in `AeroData` alongside the executable. An installed/development build uses Tauri's application-local-data folder for `com.aero.rust.browser`. Browser profiles are separate from normal Aero; cookies and site sessions are not automatically migrated.

## Build and test

Use Node.js, Rust 1.90 or newer, the Microsoft C++ build tools, and WebView2 on Windows. From this directory:

```powershell
npm ci
npm test
npm run package:portable
```

The portable output is `portable/Aero-Rust-Portable-1.0.3-x64.exe`. `npm run build` only builds the interface; it does not create an executable. `npm run dev:desktop` launches development mode. `npm run build:desktop` also builds an NSIS installer. `cargo build` from the project root now builds the real browser workspace rather than the old console demo.

## Browser controls

In **Settings > Import browser data**, choose a browser to detect its profiles automatically. Select a profile only when more than one is found, then choose bookmarks/history and press **Import selected data**. Locked history is read through a checked snapshot where possible. Passwords use a guided source-browser export followed by Aero Password Manager's authenticated import. See [IMPORT-UPDATE.md](IMPORT-UPDATE.md) for details, test results, and limitations.

The interface, styles, and original Aero icons come from the reference Aero project. Tabs support dragging, ordering, right-click actions, moving between split panes, adjustable pane widths, and audio muting. Settings stay below the window controls. Drag an empty part of the titlebar to move the window.

To avoid using the GPU, enable the graphics option in Settings and accept the restart prompt. The setting applies to the browser on whichever computer runs it. Software rendering can increase CPU use and make video less smooth.

Native tabs implement navigation, history, private windows, downloads, screenshots, MHTML saving, printing, reader mode, read-aloud, shortcuts, ad/tracker filtering, sleeping tabs, and profile clearing. Efficiency mode uses WebView2's low-memory target for background tabs without suspending audio. Clear-on-exit waits for Windows to clear website storage before closing the last normal window.

## Password manager

Open Password Manager from Aero. Configure Windows Hello in Windows Settings > Accounts > Sign-in options. Aero never asks for your Windows password. A Windows-owned, window-bound authentication prompt gates sensitive operations; unavailable or cancelled authentication fails closed.

The Rust backend uses AES-256-GCM and a random key protected by current-user Windows DPAPI. It uses the existing compatible Aero vault format at `%APPDATA%/Aero/password-vault.json`. Do not run both versions while editing that shared vault. Use encrypted backups from Password Manager; they require the original Windows DPAPI keys and are not portable recovery backups.

Passwords are saved only after an explicit login submission and user confirmation. Autofill requires explicit credential selection and fresh Windows authentication, then rechecks the HTTPS destination and visible fields in an isolated script world. Private windows cannot use the vault. Reveal, copy, export of an encrypted backup, and credential changes require fresh authentication. Clipboard clearing only removes a copied password if it still matches. Locking, sleep, minimization, inactivity, and window closure revoke authorization.

Windows Hello gates Aero operations; it does not make ordinary DPAPI encryption resistant to malware running as the same Windows user. Malware controlling the session can capture credentials. JavaScript strings cannot be reliably erased. This implementation is not claimed to be leak-proof or equivalent to Chrome/Edge; independent security review is recommended before using important accounts.

## Performance and remaining checks

Rust removes the Node/Electron host and the executable uses the system WebView2 runtime. It does not rewrite the page-rendering engine or React UI into Rust. Page speed, RAM use, smoothness, and battery life require measurement on representative websites and hardware; a Rust backend alone does not guarantee improvement.

Read [ADDRESS-BAR-FIX.md](ADDRESS-BAR-FIX.md) for the latest address-bar checks, [SETTINGS-UPDATE.md](SETTINGS-UPDATE.md) for the settings update and [REPAIR-REPORT.md](REPAIR-REPORT.md) for the initial port limitations. Real Windows Hello flows, sleep/session locking, monitor changes, and broad website compatibility still require hands-on checks. Engine-specific behavior can differ from Electron even when the Aero interface matches.

Obsolete demo sources, old executables, template assets, and an unused nested build cache were removed after backup verification. A source backup was made before changes under the Codex workspace's `work/aero-rust-original` directory. The normal Aero source was not edited by this repair.


## Settings update — 1.0.2

Password Manager is in **Privacy & shields**. Its padded description and button share one row. **Restore defaults** is also in Privacy & shields.

**Import browser data** appears directly below Performance and opens inside the existing settings frame. Performance also has an Open import button.

In **Privacy & shields > Clear browsing data**, select or deselect browsing history, download history, cookies, cached images/files, website storage, and autofill form data. Select a time range: Today (local midnight onward), the last 7/30/365 days, or All time, then choose **Clear selected data**. The toolbar flame, clear-history button, and cleanup shortcut also open this selector.

Only checked categories are cleared. Download-history clearing keeps downloaded files. Bookmarks and encrypted-vault passwords are managed separately and are kept by this cleanup. Other normal windows receive the history/download update; private browsing profiles remain separate. Website data uses WebView2's native time-range clearing and its site-level storage timestamps.

The icon has a transparent exterior in both the interface and Windows executable. The updated version has its own portable filename.

See [SETTINGS-UPDATE.md](SETTINGS-UPDATE.md) for the latest verification results, cleanup backup locations, and icon-edit provenance. The earlier REPAIR-REPORT.md describes the initial October 14 port.

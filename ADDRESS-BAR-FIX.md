# Address bar repair — 1.0.3

## Follow-up: native focus repair, version unchanged

The initial repair missed a Windows focus interruption because its tests edited a background DOM input without first focusing its native WebView2 controller. A stronger test reproduced the failure: opening the suggestion host produced blur/focus events, selected the address again, and replaced the first typed word. The earlier portable build's SHA-256 below identifies that superseded build.

The suggestion host is now created while the browser window is still hidden, before the browser accepts interaction. Input events never create a new host, even if initialization fails. Popup positioning, showing, and hiding run on the UI thread with Windows `SWP_NOACTIVATE`. Returning native focus no longer selects the address again; selection is explicit on a new click or the address shortcut. Ctrl+L/F6 now moves native focus from the webpage to Aero's interface before selecting the address, and avoids an unrelated website layout update.

The strengthened test moves focus within the test application's UI thread without bringing its offscreen window to the foreground. It clicks the address input, types a multiword phrase, and requires native focus to remain present with exactly one input focus event and zero blur events through popup opening and updates. It also tests the native address shortcut from a webpage. Synthetic profiles are used throughout.

The final `npm test` run passed all **65 checks**. The saved focus trace contains `synthetic multiword search`, native focus present, one input focus event, and zero blur events. Results are saved as `native-test-result.json` and `address-focus-report.json` in the backup/report directory below.

The portable filename and all package versions remain **1.0.3**. The previous 1.0.3 executable and edited sources were backed up under `C:\Users\rezae\Documents\Codex\2026-10-08\in-x20\work\address-bar-native-focus-20261015` before replacement.

The replacement optimized executable passed its independent launch/close check with exit code 0 and a transparent embedded icon. Its SHA-256 is `6BC2A90C776B71C7B77A519A23BD4F45ADEAFFADEF78035842A28DFE181B64A4` (17,213,440 bytes). The launch check waits for the actual browser title after WebView2 initialization.

The address bar can keep accepting a multiword search or URL while suggestions update. Suggestion updates reuse a single native popup per browser window, rather than closing and creating a WebView2 window on every edit. They no longer resize website views or broadcast browser state. Unchanged suggestion rows and bounds are not resent.

The editor preserves a draft during background URL updates, cancels pending blur timers when refocused, and dismisses suggestions when submitting or pressing Escape. Suggestion links include a generation number so stale rows cannot select a different item after an update. The popup remains isolated from browser/vault IPC.

## Verification

The native regression uses a temporary synthetic profile. It types `synthetic multiword search` in five successive text inputs while matching suggestions are present. It checks the full text and focus, verifies that the native popup handle is unchanged, sends repeated state events to check for feedback, then exercises rapid blur/refocus, Escape, typing without matches, clicking a suggestion, and pressing Enter to navigate.

The full suite now includes 4 selection/time-range tests, 19 Rust unit tests, and 42 native browser checks. No personal profile or passwords are used. Windows can emit existing invalid-window-handle diagnostics during test teardown; successful checks and a clean process exit determine the test result.

The earlier repair passed 64 weaker checks on October 15, 2026, but did not resolve the reported native focus issue. The superseded executable's SHA-256 was `04FF03ED907C25E70BED93AA499213FE47C35C3EB94A7996E611B96DE0A3A111`.

## Run

Close the old Aero Rust app and open `portable/Aero-Rust-Portable-1.0.3-x64.exe`. Keep it in the existing portable folder to continue using its `AeroData` profile. The 1.0.3 file is replaced in place; the reference Electron project and personal browsing data are preserved.

Source backups for this repair are under `C:\Users\rezae\Documents\Codex\2026-10-08\in-x20\work\address-bar-fix-20261015`.

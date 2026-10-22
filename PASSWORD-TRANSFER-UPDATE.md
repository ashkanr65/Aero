# Password Manager and export handoff — October 20, 2026

The portable version remains **1.0.3**.

## Launch repair

The suggestion popup and Password Manager previously shared WebView2's default user-data directory with different browser arguments. This is an invalid shared environment configuration, consistent with the reported `0x8007139F` error. Password Manager now has its own WebView2 UI directory, separate for software rendering. Its encrypted vault stays in the existing location and format; no credentials are migrated or reset by the repair. Native WebView2 password autosave and general autofill are explicitly disabled in this isolated UI. Failure to configure those settings closes the manager safely.

References: [Tauri data-directory requirements](https://docs.rs/tauri/latest/x86_64-pc-windows-msvc/tauri/webview/struct.WebviewWindowBuilder.html#method.data_directory), [Microsoft WebView2 environment compatibility](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/webview2-idl).

## Transfer passwords

1. Open **Settings > Import browser data**, select the browser and the intended profile, then choose **Transfer passwords from …**.
2. Aero opens Password Manager and the source browser's password settings. Check the source browser's active profile, choose **Export passwords**, and approve the source browser's confirmation/authentication. Save a **new CSV filename in Windows Downloads**.
3. Return to Aero Password Manager. Aero detects the new file automatically. Check its filename and choose **Confirm & import**, then approve Windows Hello. The native backend validates and encrypts the credentials without passing the CSV or its passwords through Aero's settings interface.
4. After a successful import, use **Unlock / refresh** if you want to review the masked credential list. Delete the readable exported CSV after checking the result.

For another save location or an existing/overwritten filename, use **Choose exported CSV…**. The original **Import CSV passwords** button also remains available. Cancel stops the transfer and never deletes an export. A failed or cancelled Windows authentication leaves the export available to retry and does not authorize a vault write.

This automates opening the managers, detecting a new export, handing it to the vault, and encrypting the import. It does **not** silently export credentials from Edge, Chrome, Firefox, or Opera, click through their confirmations, or bypass their encryption. Source-browser export and Windows authentication still require the user's approval. The source browser decides whether export is available, including under organizational policies. A CSV is readable plaintext; Aero makes no extra plaintext file, uploads nothing, and does not delete user files automatically.

## Boundaries and limits

- Watches only the current user's Windows Downloads during an explicitly started transfer, for at most ten minutes. Existing CSV paths are ignored even if overwritten. Locking, minimizing, sleep/session lock, vault closure, or cancellation revoke the transfer. Inactivity locking can end it earlier.
- Offers only new, nonempty CSV files whose size/time metadata has stopped changing. Detection does not prove the file came from the selected browser; filename confirmation is required. Multiple files remain separate choices.
- Native opaque selection tokens belong to one browser window and session. Websites and the settings renderer cannot enumerate transfer candidates, provide an arbitrary import path, or request an import. Private windows cannot start a transfer or open the vault.
- CSV reads are bounded to 8 MiB; files/reparse points are checked and the Windows handle denies concurrent writing/deletion during reading. Accepts supported browser column names, a UTF-8 BOM and CSV quoting. Limits: 5,000 valid credentials and 10,000 input rows. Only HTTPS sites with nonempty, bounded usernames/passwords are imported; unsupported rows are counted. Malformed files fail without a partial write.
- Repeated site/username pairs update the matching vault credential. Existing inaccessible/corrupt vaults fail safely. Existing AES-256-GCM, current-user DPAPI, fresh Windows Hello, authorization epochs, and atomic-write checks remain in force. Cancellation invalidates the authorization before a write can commit.
- Selected source bytes and owned sensitive JSON payloads use explicit zeroing. This does not promise complete erasure of parser allocations or JavaScript strings. Malware controlling the Windows account can capture credentials; Windows Hello is an access gate, not additional DPAPI encryption. Independent review remains advisable before trusting important accounts.

## Verification

Automated checks use synthetic credentials and isolated test profiles, never personal saved passwords. The full suite covers the existing address bar, tabs, settings, history/bookmark import, vault and browser behavior. New checks exercise actual Password Manager launch after the suggestion host is created, reopening, two simultaneous vault windows, new-file detection, existing-file exclusion, filename-only status, invalid CSV rejection, forged/cancelled/cross-window tokens, private-mode rejection, and lock cancellation. Native UI checks run with GPU acceleration enabled and disabled. CSV backend tests exercise actual Windows DPAPI and AES encryption, retry deduplication, metadata masking, revoked writes, and corrupt-vault preservation.

Tests distinguish the native backend **after** authentication from the genuine Windows Hello prompt: the encryption tests call a private test-only authenticated backend with synthetic files. No production authentication bypass was added. Actual source-browser export approval, successful/cancelled/unavailable Windows Hello on the user's configured device, and real sleep/session-lock transitions still require hands-on checks.

Build/test evidence and the previous executable are kept outside the Aero folder under `C:\Users\rezae\Documents\Codex\2026-10-08\in-x20\work\aero-password-transfer-20261020`. Generated build caches are removed after packaging to keep the project small.

Automated result: **8 frontend checks, 39 Rust tests, and 61 native browser/UI checks passed**. The 61 native checks also passed with GPU acceleration disabled. One opt-in read-only live Edge diagnosis remains ignored by default. The Windows offscreen test teardown can emit pre-existing `PostMessage` invalid-window-handle diagnostics; the completed reports passed and processes exited successfully.

Profile preservation check: all **1,190 readable pre-existing AeroData files** matched their original SHA-256 hashes. Fourteen active WebView2/cache files could not be hashed, so no byte-for-byte preservation claim is made for those. Tests did not use the personal profile or the personal vault for imports.

Portable verification: the optimized build completed successfully, and the staged executable started and closed normally with an isolated synthetic profile both before and after removing build dependencies. Its embedded icon has a transparent corner. Current executable: `portable/Aero-Rust-Portable-1.0.3-x64.exe`, **17,448,960 bytes**, SHA-256 **A73E0BA0DF68CD8A8AB17A07EBF523B3BBFB6C73AB9B4E110B8C89D75ED4A89E**. Approximately **5 GB** of generated dependencies, compiled caches, and synthetic profiles were removed after verification. Source, lockfiles, tests, Git history, the current portable, and personal AeroData were retained.

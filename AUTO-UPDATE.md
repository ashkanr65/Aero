# Automatic updates — Aero 1.0.4

Users of the updater-enabled portable do not need to choose an update button, download an installer, or approve an Aero restart. Aero checks the public stable-release feed quietly after startup and periodically while running. A verified new version is downloaded in the background. The current browsing session stays open. The next ordinary launch automatically starts the downloaded version from the same shortcut and uses the original profile.

**One-time migration:** 1.0.3 and earlier do not contain an updater. Those users must receive the updater-enabled build once. A running browser cannot add update code to an older copy on someone else's computer.

## Where releases come from

Repository: `https://github.com/ashkanr65/Aero`.

Feed: `https://github.com/ashkanr65/Aero/releases/latest/download/latest.json`, with detached signature at `latest.json.sig`. End users authenticate neither to GitHub nor to Aero. GitHub account tokens are never embedded in the browser. The repository must be public for these anonymous release URLs.

Release verification uses Ed25519 through maintained Node/OpenSSL and `ed25519-dalek` implementations, plus SHA-256 for binding the exact artifact bytes. Only the public verification key is compiled into Aero. The signed manifest binds the schema, stable semantic version, Windows x64 target, exact repository release URL, size, and artifact hash. Invalid signatures, changed bytes, mismatched lengths, wrong architecture, unrelated URLs, unstable versions, and downgrade/replayed older releases are rejected. Downloads use certificate-verified HTTPS with constrained GitHub redirects and bounded sizes/timeouts.

## Activation and recovery

The original executable remains a stable launch entry. Verified payloads live in a hidden `.aero-updates/versions/<version>/Aero.exe` directory beside it. Only a small metadata pointer changes atomically after a complete verified download. This avoids overwriting an executable while Windows is running it. Files left by an interrupted download cannot become active.

On launch, Aero verifies the cached signature and executable again, starts the selected payload, and waits for its startup acknowledgement. A failed launch or missing/wrong acknowledgement rejects that version and tries the previous working payload or the original executable. A later release can still update normally. An exclusive per-installation download lock prevents two Aero processes from staging or cleaning one update concurrently. The launcher holds the verified executable open against writing/deletion until the new process is created.

Portable `AeroData` remains beside the original launch entry. The new payload inherits the validated original path rather than creating a new profile inside its cache. Installed per-user profiles and the existing `%APPDATA%/Aero/password-vault.json` path stay unchanged. The update code never reads, migrates or deletes credentials, bookmarks, history, or settings. It does not remove unknown files during cache cleanup. The active version and previous payload are retained; older unused payloads and incomplete downloads are cleaned when safe. Windows can keep an older payload locked until its last process exits.

There is no immediate forced restart, background Windows service, scheduled task, UAC bypass, or website-controlled update command. Checks begin about ten seconds after opening Aero, repeat about every six hours after success, and retry after about fifteen minutes on failure. Offline use and failed checks keep the working version. If the browser was closed when a release appeared, it checks after being opened; a downloaded update becomes active on the following launch. Settings > Performance shows the running version and update status.

## Publishing future updates

The private release-signing key remains on the publisher's Windows computer at `%LOCALAPPDATA%/AeroPublisher/update-signing.dpapi`, protected with current-user Windows DPAPI. It is not in Git, a release asset, a browser binary, or GitHub Actions secrets. Only signatures and public artifacts are uploaded. Preserve the publisher's Windows profile/DPAPI keys and protected key file: losing this identity prevents signing updates trusted by existing launchers. Key rotation or loss requires a planned migration; do not replace the pinned key casually.

GitHub Actions runs release/build checks on every push and on every version tag. When the repository holds an `AERO_UPDATE_PRIVATE_KEY` secret, the tagged workflow signs the portable and publishes the complete signed release, which is the same result the local publisher produces. Without that secret the workflow still publishes the portable to the repository's Releases section, but as an unsigned build that stays out of the update feed, so Aero keeps updating from the signed latest release. An unsigned CI release must be deleted before the same version can be published as a signed release, because a public release is never overwritten.

Publishing uses the existing Git credential manager locally. From the publisher computer:

```powershell
npm ci
npm run release:version -- 1.0.5
# Make the intended changes, then commit all release source.
git add <the intended source files>
git commit -m "Release Aero 1.0.5"
git tag v1.0.5
git push origin main v1.0.5
npm run release:publish
```

The local release command checks the source is committed/tagged, runs regression tests, builds the exact portable, signs the manifest/artifact locally, clears private-key environment variables, and uploads only public files. GitHub credentials are used only for GitHub API authorization. The release is kept as a draft while assets are uploaded, then made public/latest together. Re-running cannot overwrite an already public version; increase the version for another release. Keep the version-1 manifest contract and signing identity compatible with deployed launchers.

Both the browser and updater are Windows x64. Automatic writes require a writable application folder: the default per-user installer and normal portable folders support this. Read-only media, protected system directories, managed-device policies, antivirus blocking, or lack of storage can prevent an update; Aero keeps the working version and retries. Ed25519 update signing is separate from Windows Authenticode/SmartScreen reputation. It does not protect against malware already controlling the Windows account. Future data-format migrations must remain compatible with the advertised fallback policy.

## Verification

Tests use isolated synthetic fixtures, not the personal profile or passwords. The suite covers strict signature verification, trusted destinations, versions and replay protection, truncated/tampered downloads, damaged-cache repair, atomic metadata failure, exclusive locks, preservation of unknown/profile files, fallback selection, and real executable handoff from a path containing spaces. A signed fixture starts on the next launch while the old session marker remains unchanged during staging. A payload that reports the wrong compiled version fails its startup check and falls back. Separate browser tests check update status and rejection of arbitrary renderer update requests alongside existing tabs/settings/import/vault regressions.

Test and build evidence is outside the repository under `C:\Users\rezae\Documents\Codex\2026-10-08\in-x20\work\aero-updater-20261020`. Source/executable backups are there as well. Real Windows authentication remains separate from update testing; no password-vault access gate was weakened.

Primary references: [Tauri updater signing and Windows installer support](https://v2.tauri.app/plugin/updater/), [ed25519-dalek](https://docs.rs/ed25519-dalek/latest/ed25519_dalek/), [Node crypto](https://nodejs.org/api/crypto.html), [GitHub release API](https://docs.github.com/en/rest/releases/releases).

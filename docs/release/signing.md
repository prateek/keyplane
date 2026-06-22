# Signed Release Workflow

The `Signed Release` GitHub Actions workflow builds a signed macOS `.app` bundle
and `.dmg` artifact when a `v*` tag is pushed or when the workflow is started
manually.

This is a release scaffold, not proof that the first signed release has shipped.
The workflow still needs to be run with real Apple signing credentials before
signed release packaging can be treated as validated.

## Required Secrets

Configure these GitHub Actions secrets before running the workflow:

- `APPLE_CERTIFICATE`: base64-encoded `.p12` signing certificate
- `APPLE_CERTIFICATE_PASSWORD`: password for the exported `.p12`
- `APPLE_ID`: Apple ID email for notarization
- `APPLE_PASSWORD`: app-specific password for that Apple ID
- `APPLE_TEAM_ID`: Apple team ID for notarization
- `KEYCHAIN_PASSWORD`: temporary CI keychain password

The workflow imports the certificate into a temporary macOS keychain, selects the
first available Apple code-signing identity, exports it as
`APPLE_SIGNING_IDENTITY`, and runs:

```sh
npm run tauri build -- --bundles app,dmg --ci
```

This follows Tauri's macOS signing and notarization environment-variable
contract:

- <https://v2.tauri.app/distribute/sign/macos/>

Manual runs default to `skip_stapling=true`, which appends `--skip-stapling`.
Set `skip_stapling=false` after notarization credentials are confirmed and the
release should fail if stapling fails.

## Local Dry Checks

The workflow cannot be fully proven without Apple credentials, but its static
shape should stay validated with:

```sh
npm run check:workflows
actionlint .github/workflows/signed-release.yml .github/workflows/desktop-build.yml
```

The `Desktop Build` PR workflow runs `npm run check:workflows` so the signed
release scaffold and evidence collectors stay checked before real Apple
credentials are available.

The unsigned/debug build path remains covered by the `Desktop Build` workflow.
Windows and Linux signed installers are intentionally not part of this scaffold;
those platforms need separate certificate/provider decisions.

## Signed Run Evidence

After the first real signed run completes with Apple credentials, collect a
sanitized evidence report from the GitHub Actions run:

```sh
npm run validate:signed-release
```

By default, the collector looks up the latest completed `Signed Release` run in
GitHub Actions. This works after the workflow file exists on the repository's
default branch. To inspect a specific run instead, set
`KEYPLANE_SIGNED_RELEASE_RUN_ID` or pass `--run-id`.

The report is written to `target/keyplane-validation/signed-release.md`. It
checks that the `Signed Release` workflow completed successfully, that the
`Signed macOS release` job passed, and that both signed artifact records exist:

- `keyplane-macos-signed-app`
- `keyplane-macos-signed-dmg`

To check report generation without querying GitHub:

```sh
npm run validate:signed-release:dry
```

The dry run is not release evidence. Do not paste Apple credentials, signing
identities, or notarization secrets into PR comments or committed files.

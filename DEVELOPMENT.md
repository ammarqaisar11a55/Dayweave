# Development

## Requirements

- Rust stable
- Node.js 22
- Ubuntu's Tauri dependencies:

```bash
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libgtk-3-dev patchelf
```

## Layout

| Path | Holds |
| --- | --- |
| `crates/dayweave-core` | Storage and all product logic |
| `src-tauri` | The desktop shell: window, tray, scheduler, notifications |
| `src` | The React interface |
| `tests/gnome` | Checks and screenshots against a real, disposable GNOME Shell |
| `snap`, `.github` | Packaging and CI |

`crates/dayweave-core` builds and tests without GTK or WebKit, so most work needs no desktop
libraries. [ARCHITECTURE.md](ARCHITECTURE.md) explains how the pieces fit together.

## Running

The desktop application:

```bash
npm install
```

```bash
npm run tauri dev
```

The interface in an ordinary browser, against the real core and a real database. This is the
quickest way to work on the interface:

```bash
npm run bridge -- /tmp/dayweave-dev 1421
```

```bash
npm run dev
```

Then open <http://localhost:1420>. The bridge is a development tool behind the `dev-bridge` Cargo
feature. It listens on localhost only and is never packaged.

## Tests

```bash
cargo test -p dayweave-core
```

```bash
npm test
```

```bash
npm run lint && npm run typecheck
```

The desktop check runs the real binary in a headless GNOME Shell with its own D-Bus, home and
settings, so it never touches your session. It covers the window, persistence, single instance,
closing to the background, and a real notification:

```bash
cargo build -p dayweave -F tauri/custom-protocol
```

```bash
cargo build -p dayweave-core --features dev-bridge --bin dayweave-bridge
```

```bash
tests/gnome/session.sh python3 tests/gnome/desktop.py target/debug/dayweave target/debug/dayweave-bridge
```

## Screenshots

The README's images come from the real application in that same disposable session, reached with
its own keyboard shortcuts. After any visual change, build as above, then regenerate them:

```bash
tests/gnome/session.sh python3 tests/gnome/screenshots.py target/debug/dayweave target/debug/dayweave-bridge docs/screenshots
```

## Changing the schema

Add a new file in `crates/dayweave-core/src/migrations`, list it in `MIGRATIONS` in `db.rs`, and add
any new table to `TABLES`, where a test checks that nothing is missed. Never edit a migration that
has been released.

## Secrets

| Secret | Purpose |
| --- | --- |
| `SNAPCRAFT_STORE_CREDENTIALS` | Publishes to the Snap Store. Create it with `snapcraft export-login` |

Credentials belong in GitHub Secrets and must never be committed.

## Releasing

1. Update the version in `package.json`, `src-tauri/tauri.conf.json`, the root `Cargo.toml` and the
   metainfo file.
2. Update `CHANGELOG.md`.
3. Push a tag such as `v1.2.0`, or `v1.2.0-beta.1` for a prerelease.

The release workflow checks that the tag matches, runs the tests, and builds the `.deb`, AppImage
and snap into a **draft** GitHub release. Nothing is public until you publish the draft.

To publish a `.deb` that CI already built as a pre-release, without rebuilding it, run the
**Publish .deb** workflow with the tag and the ID of the CI run that built it.

To put a snap on a Snap Store channel, run the **Publish snap** workflow and choose `edge`, `beta`,
`candidate` or `stable`. It uses the release's snap, or the snap from a workflow run if you give one. Create two GitHub environments first: `snap-prerelease`,
and `snap-stable` with a required reviewer, which is what makes a stable release wait for approval.

## Snap builds

GitHub Actions builds the snap on every relevant change, so most work doesn't need a local build.
To build locally, install LXD once:

```bash
sudo snap install lxd
```

```bash
sudo lxd init --auto
```

Then:

```bash
snapcraft pack
```

Strict confinement can behave differently from a normal run, so try tray, autostart and exports in
the real snap before a stable release.

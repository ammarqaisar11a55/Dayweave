# Privacy

Dayweave keeps everything on your computer.

- **No account.** There is nothing to sign up for.
- **No cloud.** Nothing is synchronised or uploaded.
- **No telemetry.** Dayweave doesn't measure how you use it.
- **No network.** It makes no network requests at all. Its content security policy allows only the
  internal channel between its interface and its core, and the snap doesn't declare the `network`
  permission.
- **No advertising or third-party tracking.** No third-party services are involved. The Inter
  typeface ships inside the application, so not even a font is fetched.

## Your data

All your data is in one SQLite database in your user data folder, which Settings shows. It is not
encrypted, so it has the same protection as your other files. Anyone with access to your user
account can read it.

Automatic safety copies are kept next to it in a `backups` folder: one before each restore, up to
the last five. One more is kept beside the database before a schema upgrade.

Backups and exports are ordinary files. You decide where they go, and what happens to them after
that is up to you.

## Notifications

Reminders appear in your desktop's notification list with the task or habit's name in them. If you
share your screen or leave your computer unlocked, others may see them. You can turn them off in
Settings, and the in-app notification list keeps working.

## Snap permissions

| Permission | Why |
| --- | --- |
| `desktop`, `desktop-legacy`, `wayland`, `x11`, `opengl`, `gsettings` | Show a window, follow the desktop theme and send notifications. These come from the GNOME extension |
| `browser-support` | WebKitGTK, which draws the interface, needs it to run |

The snap doesn't request `network` or `home`.

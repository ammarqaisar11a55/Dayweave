# Privacy

Dayweave keeps everything on your computer unless you turn on sync.

- **No account needed.** Dayweave works fully without one.
- **No telemetry.** Dayweave doesn't measure how you use it.
- **No network unless you sync.** Without sync, Dayweave makes no network requests at all. Its
  interface can't either: the content security policy allows only the internal channel between the
  interface and the core.
- **No advertising or third-party tracking.** No third-party services are involved. The Inter
  typeface ships inside the application, so not even a font is fetched.

## Sync

Sync is off until you sign in to a Dayweave server in Settings → Sync. Then:

- Dayweave talks only to the server address you entered, over HTTPS, and only to sign in, sync and
  sign out. It refuses plain HTTP except to a server on your own computer.
- Your tasks, habits, check-ins, projects, finished focus sessions and preferences are sent to that
  server and stored there, so your other devices can fetch them. Start at login, keep running in the
  background and the sidebar state stay on this computer, as does the notification list.
- The server stores your email and a hash of your password. Your password itself is never stored
  on your computer: Dayweave keeps a token for this device instead, in its database.
- Backups and database copies never contain that token.
- Signing out stops syncing and revokes this device's token. Your data stays on the computer and on
  the server. **Delete account** removes everything stored on the server.

Whoever runs the server can read what it stores. Use a server you trust, or run your own.

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
| `network` | Only for sync, to reach the server you chose. Nothing uses it while sync is off |

The snap doesn't request `home`.

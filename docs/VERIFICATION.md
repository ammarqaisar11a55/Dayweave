# Verification status

This records what has actually been run. A feature isn't called working because its code compiles.
Last updated 2026-11-02 for version 0.1.0, on Ubuntu 26.04 with GNOME Shell 50.

## Results

| Suite | Result |
| --- | --- |
| Core tests | 187 passed |
| Desktop shell tests | 3 passed |
| Interface tests | 31 passed |
| Real application in GNOME, on the release binary | 17 of 17 checks passed |
| Lint, formatting, type checks and Clippy with warnings as errors | Clean |
| GitHub Actions CI on Ubuntu 24.04 | All three jobs passed |
| Snap build on GitHub Actions | Succeeded in about 10 minutes |
| Publish .deb workflow | Published v0.1.0 as a pre-release |
| Publish snap workflow | Published revision 1 to the beta channel, and the store's automated review passed |
| Snap on a real desktop | Revision 1 was installed from the beta channel and tested by hand by the author, then released to stable. The individual checks weren't recorded |

The release binary is 7.1 MB, the `.deb` is 3.3 MB, and the AppImage is 85 MB because it carries WebKit.

## Verified by running it

| Area | How |
| --- | --- |
| Tasks, deadlines, overdue detection, sorting, filters | Integration tests through the same dispatcher the interface uses |
| Recurring tasks | Unit and integration tests, including month ends, leap days, skipped occurrences and reopening |
| Subtasks and projects | Integration tests, including nesting and deletion |
| Habits and streaks | Unit tests of the streak engine and integration tests for all four kinds and schedules |
| Reminders | Integration tests of firing, rescheduling, cancelling, snoozing, missed reminders and a clock moving backwards |
| Timezones and daylight saving | Tests against real zones, including skipped and repeated hours |
| Focus sessions | Integration tests of pausing, restarts and finishing while closed |
| Backup and restore | Round trips in both formats, plus broken references, invalid values, SQL in column names, damaged files, other applications' databases and newer versions. Live data is checked as unchanged after each rejection |
| Exports to a file with no writable directory | Tested with a read-only directory, which behaves like a file granted by a portal |
| Autostart entry and single-instance socket | Unit tests of the real code, including a stale socket |
| Interface | Driven by hand in a browser against the real core: every page in both themes, task completion and undo, the details panel, keyboard task creation, drag to reschedule, focus start and pause, a first run on an empty database, and a validation error |
| Window on GNOME and Wayland | The real binary in a real headless GNOME Shell. See `tests/gnome/desktop.py` |
| Single instance | A second launch hands over and exits, and only one window exists |
| Launcher actions | `--new-task` reached the running copy and opened the dialog |
| Closing to the background | The window hides, the process keeps running, and launching again reopens it |
| Desktop notification | A reminder reached GNOME's message tray with the window closed, was recorded once, and did not repeat |
| Notification buttons | Complete was pressed through GNOME and finished the task |
| Keyboard shortcuts in the real window | Ctrl+1 to 7, Ctrl+K, Ctrl+comma, J and Enter, typed through a virtual keyboard while taking the screenshots |
| Colour contrast and chart palette | Ratios computed for all text colours. Chart colours pass a colour-vision validator in both themes |
| AppImage | Launched in a real headless GNOME Shell. The window opened and the database was created |
| Metadata | `desktop-file-validate` and `appstreamcli validate` pass, and `snapcraft expand-extensions` accepts the recipe |

## Compiled or built, not run

| Area | Status |
| --- | --- |
| `.deb` | Built, with its contents, dependencies and launcher inspected. Not installed, because that needs root |
| Release workflow | Its checks and all three builds passed for v0.1.0, but its last step failed. That step is fixed and has not run again |

## Not verified

| Area | Why |
| --- | --- |
| Tray icon and menu | A headless session has no AppIndicator host, so this has no automated check |
| Snooze button | Only Complete was pressed. Snooze is covered by an integration test of the same operation |
| File dialogs | The save and open dialogs were never opened, because a headless session has no portal backend for them. The export and restore code behind them is tested |
| Start at login after a real sign-in | The entry is written correctly, but no real session has started from it |
| Theme following a live desktop change | Light and dark were both seen. Switching while running in `system` mode wasn't |
| Real suspend and resume | Handled by the stored last-check time and tested with a simulated clock |
| A second schema version | Migrations are written for it, but only version 1 exists, so no real upgrade has run. The refusal of a newer database is tested |
| Screen readers | Labels and roles are in place, but no screen reader has been used |
| Other Ubuntu versions and other desktops | Only Ubuntu 26.04 with GNOME 50 was available |

## Running the suites

```bash
cargo test -p dayweave-core
```

```bash
cargo test -p dayweave
```

```bash
npm test
```

```bash
tests/gnome/session.sh python3 tests/gnome/desktop.py target/debug/dayweave target/debug/dayweave-bridge
```

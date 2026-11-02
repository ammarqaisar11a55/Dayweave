#!/usr/bin/env python3
"""Checks the real application in a real GNOME Shell. Run inside session.sh.

usage: tests/gnome/session.sh python3 tests/gnome/desktop.py <dayweave> <dayweave-bridge> [screenshot-dir]
"""

import os
import signal
import sqlite3
import subprocess
import sys
import time
from datetime import datetime, timedelta
from pathlib import Path

from common import PORT, day, rpc, shell_eval, wait_for, wait_for_bridge, windows

APP, BRIDGE = sys.argv[1], sys.argv[2]
SHOTS = Path(sys.argv[3]).resolve() if len(sys.argv) > 3 else None
IDENTIFIER = "io.github.ammarqaisar11a55.dayweave"
DATA = Path(os.environ["XDG_DATA_HOME"]) / IDENTIFIER
DATABASE = DATA / "dayweave.db"

failures = 0


def check(name, passed, detail=""):
    global failures
    print(f"{'PASS' if passed else 'FAIL'}  {name}{f'  ({detail})' if detail and not passed else ''}", flush=True)
    failures += 0 if passed else 1


def screenshot(name):
    if SHOTS:
        subprocess.run(
            ["gdbus", "call", "--session", "-d", "org.gnome.Shell", "-o", "/org/gnome/Shell/Screenshot",
             "-m", "org.gnome.Shell.Screenshot.Screenshot", "false", "false", str(SHOTS / name)],
            capture_output=True, check=False,
        )


def seed():
    """Fills the database through the same dispatcher the application uses."""
    bridge = subprocess.Popen([BRIDGE, str(DATA), str(PORT)], stdout=subprocess.DEVNULL)
    wait_for_bridge()
    project = rpc("projects.create", {"name": "Final year thesis", "color": "blue", "icon": "graduation-cap"})
    rpc("tasks.create", {"title": "Prepare Software Engineering Assignment", "priority": "high", "dueDate": day(0),
                         "dueTime": "23:30", "projectId": project["id"], "subtasks": ["Research topic", "Write up"]})
    rpc("tasks.create", {"title": "Submit scholarship documents", "priority": "urgent", "dueDate": day(-1)})
    rpc("tasks.create", {"title": "Book dentist appointment", "dueDate": day(1), "dueTime": "11:00"})
    done = rpc("tasks.create", {"title": "Review pull request", "dueDate": day(0)})
    rpc("tasks.complete", {"id": done["id"]})
    habit = rpc("habits.create", {"name": "Study DSA", "icon": "brain", "color": "blue", "startDate": day(-30)})
    rpc("habits.create", {"name": "Read", "kind": "quantity", "quantityTarget": 20, "unit": "pages", "icon": "book-open",
                          "color": "amber", "startDate": day(-30)})
    for offset in range(-14, 1):
        rpc("habits.log", {"id": habit["id"], "date": day(offset)})

    # One minute of margin so the application has started before this falls due.
    moment = (datetime.now() + timedelta(seconds=120)).strftime("%Y-%m-%dT%H:%M")
    rpc("tasks.create", {"title": "Reminder under test", "dueDate": day(30),
                         "reminders": [{"kind": "at", "remindAt": moment}]})
    rpc("settings.set", {"key": "theme", "value": os.environ.get("DAYWEAVE_TEST_THEME", "dark")})
    bridge.terminate()
    bridge.wait()
    # The scheduler measures from its last pass, so the reminder must fall due after this point.
    return moment


def tray_notifications():
    code = "JSON.stringify(Main.messageTray.getSources().flatMap(s => s.notifications.map(n => n.title)))"
    return shell_eval(code) or []


def press_notification_button(label):
    shell_eval(
        "Main.messageTray.getSources().flatMap(s => s.notifications)"
        ".filter(n => n.title === 'Reminder under test')"
        f".forEach(n => n.actions.find(a => a.label === '{label}').activate()); 'ok'"
    )


def task_status(title):
    connection = sqlite3.connect(f"file:{DATABASE}?mode=ro", uri=True)
    row = connection.execute("SELECT status FROM tasks WHERE title = ?", (title,)).fetchone()
    connection.close()
    return row[0] if row else ""


def main():
    moment = seed()
    log = open(Path(os.environ["DAYWEAVE_TEST_WORK"]) / "app.log", "w")
    app = subprocess.Popen([APP], stdout=log, stderr=log)

    found = wait_for(windows, 25)
    check("the window opens in GNOME on Wayland", bool(found), str(found))
    if found:
        check("the window has the application's title", found[0]["title"] == "Dayweave", found[0]["title"])
        # The launcher's StartupWMClass has to match this for the dock to group the window with it.
        check("the window matches the launcher entry", found[0]["app"] == "dayweave", str(found[0]))
    check("the database is created in the XDG data directory", DATABASE.exists(), str(DATABASE))
    time.sleep(2.5)
    screenshot("overview.png")

    connection = sqlite3.connect(f"file:{DATABASE}?mode=ro", uri=True)
    rows = connection.execute("SELECT count(*) FROM tasks").fetchone()[0]
    check("data written earlier is still there", rows == 5, str(rows))
    connection.close()

    second = subprocess.run([APP, "--new-task"], capture_output=True, timeout=10)
    check("a second launch hands over and exits", second.returncode == 0, str(second.returncode))
    time.sleep(1)
    check("only one window exists after a second launch", len(windows()) == 1, str(windows()))
    check("the first process is still the running copy", app.poll() is None)
    screenshot("new-task.png")

    shell_eval(
        "global.get_window_actors().map(a => a.meta_window)"
        ".filter(w => w.get_title() === 'Dayweave').forEach(w => w.delete(global.get_current_time()))"
    )
    hidden = wait_for(lambda: len(windows()) == 0, 5)
    check("closing the window hides it", hidden, str(windows()))
    time.sleep(1)
    check("the application keeps running after the window closes", app.poll() is None)

    fire = datetime.strptime(moment, "%Y-%m-%dT%H:%M")
    remaining = (fire - datetime.now()).total_seconds()
    print(f"      waiting {max(0, remaining):.0f}s for a reminder with the window closed", flush=True)
    seen = wait_for(lambda: "Reminder under test" in tray_notifications(), remaining + 45)
    check("a reminder reaches GNOME while the window is closed", bool(seen), str(tray_notifications()))

    connection = sqlite3.connect(f"file:{DATABASE}?mode=ro", uri=True)
    logged = connection.execute(
        "SELECT count(*) FROM notification_log WHERE title = 'Reminder under test'"
    ).fetchone()[0]
    connection.close()
    check("the reminder is recorded exactly once", logged == 1, str(logged))
    time.sleep(25)
    connection = sqlite3.connect(f"file:{DATABASE}?mode=ro", uri=True)
    logged = connection.execute(
        "SELECT count(*) FROM notification_log WHERE title = 'Reminder under test'"
    ).fetchone()[0]
    connection.close()
    count = tray_notifications().count("Reminder under test")
    check("it does not repeat on the next pass", logged == 1 and count == 1, f"{logged} {count}")

    press_notification_button("Complete")
    completed = wait_for(lambda: task_status("Reminder under test") == "completed", 6)
    check("the notification's Complete button finishes the task", completed, task_status("Reminder under test"))

    subprocess.run([APP], capture_output=True, timeout=10)
    reopened = wait_for(windows, 8)
    check("launching again reopens the hidden window", bool(reopened), str(reopened))
    check("launching again starts no second copy", app.poll() is None and len(windows()) == 1)

    app.send_signal(signal.SIGTERM)
    app.wait(timeout=5)
    leftover = (Path(os.environ["XDG_RUNTIME_DIR"]) / "dayweave.sock").exists()
    relaunched = subprocess.Popen([APP], stdout=log, stderr=log)
    back = wait_for(windows, 25)
    check("a stale socket from a killed process doesn't block the next start", bool(back) and leftover, str(leftover))
    relaunched.send_signal(signal.SIGTERM)
    relaunched.wait(timeout=5)

    print(f"\n{'All checks passed' if not failures else f'{failures} checks failed'}", flush=True)
    if failures:
        print((Path(os.environ["DAYWEAVE_TEST_WORK"]) / "app.log").read_text()[-1500:])
    return 1 if failures else 0


sys.exit(main())

#!/usr/bin/env python3
"""Regenerates the README screenshots from the real application. Run inside session.sh.

Pages are reached with the application's own keyboard shortcuts, typed through a virtual keyboard.

usage: tests/gnome/session.sh python3 tests/gnome/screenshots.py <dayweave> <dayweave-bridge> <output-dir>
"""

import os
import random
import shutil
import sqlite3
import subprocess
import sys
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path

from common import PORT, day, rpc, shell_eval, wait_for, wait_for_bridge, windows

APP, BRIDGE, OUTPUT = sys.argv[1], sys.argv[2], Path(sys.argv[3]).resolve()
DATA = Path(os.environ["XDG_DATA_HOME"]) / "io.github.ammarqaisar11a55.dayweave"
DATABASE = DATA / "dayweave.db"

KEYS = """
const Clutter = imports.gi.Clutter;
const GLib = imports.gi.GLib;
const seat = global.stage.get_context().get_backend().get_default_seat();
// One device for the whole run. A new one takes a moment to register, so its first keys land late.
global.dayweaveKeyboard ??= seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
const keyboard = global.dayweaveKeyboard;
const send = (keyval, state) => keyboard.notify_keyval(GLib.get_monotonic_time(), keyval, state);
const press = (keyval) => {
    send(keyval, Clutter.KeyState.PRESSED);
    send(keyval, Clutter.KeyState.RELEASED);
};
"""


def seed_people_data():
    """A plausible few months, written through the same dispatcher the application uses."""
    thesis = rpc("projects.create", {"name": "Final year thesis", "description": "Distributed scheduling", "color": "blue",
                                     "icon": "graduation-cap", "startDate": day(-30), "deadline": day(45)})
    site = rpc("projects.create", {"name": "Portfolio site", "description": "Refresh before job applications",
                                   "color": "violet", "icon": "code", "deadline": day(12)})
    home = rpc("projects.create", {"name": "Home", "color": "green", "icon": "home"})

    dsa = rpc("habits.create", {"name": "Study DSA", "icon": "brain", "color": "blue", "startDate": day(-200),
                                "reminderTime": "20:00"})
    read = rpc("habits.create", {"name": "Read", "kind": "quantity", "quantityTarget": 20, "unit": "pages",
                                 "icon": "book-open", "color": "amber", "startDate": day(-200)})
    water = rpc("habits.create", {"name": "Drink water", "kind": "quantity", "quantityTarget": 2, "unit": "liters",
                                  "icon": "droplets", "color": "teal", "startDate": day(-150)})
    smoking = rpc("habits.create", {"name": "Avoid smoking", "kind": "quit", "icon": "wind", "color": "red",
                                    "startDate": day(-60)})
    run = rpc("habits.create", {"name": "Run", "frequency": "weekly_target", "weeklyTarget": 3, "icon": "footprints",
                                "color": "green", "startDate": day(-120)})
    routine = rpc("habits.create", {"name": "Morning routine", "kind": "checklist", "icon": "sun", "color": "orange",
                                    "checklist": [{"title": "Stretch"}, {"title": "Plan the day"}, {"title": "Make the bed"}],
                                    "startDate": day(-60)})
    gym = rpc("habits.create", {"name": "Gym", "frequency": "weekdays", "weekdays": [1, 3, 5], "icon": "dumbbell",
                                "color": "pink", "startDate": day(-110)})

    generator = random.Random(7)
    for offset in range(-199, 0):
        trend = 0.55 + 0.38 * (offset + 199) / 199
        if generator.random() < trend:
            rpc("habits.log", {"id": dsa["id"], "date": day(offset)})
        if generator.random() < trend:
            pages = 20 + generator.randrange(15) if generator.random() < 0.8 else 8 + generator.randrange(10)
            rpc("habits.log", {"id": read["id"], "date": day(offset), "value": pages})
        if offset >= -150 and generator.random() < trend:
            rpc("habits.log", {"id": water["id"], "date": day(offset), "value": 2 if generator.random() < 0.8 else 1.25})
        if offset >= -120 and generator.random() < 0.45:
            rpc("habits.log", {"id": run["id"], "date": day(offset)})
        if offset >= -110 and generator.random() < trend:
            rpc("habits.log", {"id": gym["id"], "date": day(offset)})
        if offset >= -60 and generator.random() < trend:
            items = [item["id"] for item in routine["checklist"]]
            rpc("habits.log", {"id": routine["id"], "date": day(offset), "checkedItems": items})

    rpc("habits.log", {"id": smoking["id"], "date": day(-23), "slip": True, "note": "Exam week"})
    rpc("habits.log", {"id": dsa["id"]})
    rpc("habits.log", {"id": water["id"], "value": 1.25})
    rpc("habits.log", {"id": read["id"], "value": 12})
    rpc("habits.log", {"id": routine["id"], "checkedItems": [item["id"] for item in routine["checklist"][:2]]})

    before = [{"kind": "before", "offsetMinutes": 30}]
    assignment = rpc("tasks.create", {
        "title": "Prepare Software Engineering Assignment", "projectId": thesis["id"], "priority": "high",
        "dueDate": day(0), "dueTime": "23:30", "tags": ["university"], "category": "University", "reminders": before,
        "description": "Weekly submission for the software engineering course.",
        "subtasks": ["Research topic", "Write introduction", "Implement solution", "Prepare screenshots", "Submit assignment"],
    })
    subtasks = rpc("tasks.get", {"id": assignment["id"]})["subtasks"]
    rpc("subtasks.toggle", {"id": subtasks[0]["id"]})
    rpc("subtasks.toggle", {"id": subtasks[1]["id"]})

    task = lambda title, **extra: rpc("tasks.create", {"title": title, **extra})
    task("Pay electricity bill", dueDate=day(0), priority="urgent", category="Finance",
         reminders=[{"kind": "before", "offsetMinutes": 0}], recurrence={"frequency": "monthly"})
    task("Review pull request from Ayesha", projectId=site["id"], dueDate=day(0), tags=["code"])
    task("Call the landlord about the heater", projectId=home["id"], dueDate=day(0), priority="low")
    task("Submit scholarship documents", dueDate=day(-2), priority="high", tags=["university"])
    task("Update resume", dueDate=day(-1), projectId=site["id"])
    task("Book dentist appointment", dueDate=day(1), dueTime="11:00", projectId=home["id"])
    task("Draft thesis chapter 3", startDate=day(1), dueDate=day(4), projectId=thesis["id"], priority="high",
         estimatedMinutes=240, tags=["writing", "university"])
    task("Deploy portfolio preview", dueDate=day(3), projectId=site["id"], tags=["code"])
    task("Weekly review", dueDate=day(5), dueTime="18:00", recurrence={"frequency": "weekly"})
    task("Renew library card", dueDate=day(9), priority="low")
    task("Prepare viva slides", dueDate=day(21), projectId=thesis["id"], priority="high")
    task("Clean up old project notes", priority="low", category="Admin")

    done = rpc("tasks.create", {"title": "Fix navbar on mobile", "projectId": site["id"], "dueDate": day(0)})
    rpc("tasks.complete", {"id": done["id"]})
    for title, project in [("Set up CI for thesis code", thesis), ("Outline chapter 2", thesis), ("Buy a notebook", home)]:
        created = rpc("tasks.create", {"title": title, "projectId": project["id"]})
        rpc("tasks.complete", {"id": created["id"]})
    return thesis["id"], site["id"], generator


def backdate_history(thesis, site, generator):
    """Completion dates and focus sessions can only happen now, so past ones are written directly."""
    now = datetime.now(timezone.utc)
    connection = sqlite3.connect(DATABASE)
    for offset in range(-119, 0):
        stamp = (now + timedelta(days=offset)).strftime("%Y-%m-%dT%H:%M:%SZ")
        for index in range(generator.choice([0, 1, 1, 2, 2, 3])):
            connection.execute(
                "INSERT INTO tasks (title, status, priority, project_id, created_at, updated_at, completed_at, completed_on) "
                "VALUES (?, 'completed', 'medium', ?, ?, ?, ?, ?)",
                (f"Finished task {offset} {index}", generator.choice([thesis, site, None]), stamp, stamp, stamp, day(offset)),
            )
        for _ in range(generator.choice([0, 1, 1, 2, 3])):
            connection.execute(
                "INSERT INTO focus_sessions (kind, status, planned_seconds, elapsed_seconds, local_date, started_at, ended_at) "
                "VALUES ('focus', 'completed', 1500, 1500, ?, ?, ?)",
                (day(offset), stamp, stamp),
            )
    connection.commit()
    connection.close()


def press(*keys, control=False):
    code = [KEYS]
    if control:
        code.append("send(Clutter.KEY_Control_L, Clutter.KeyState.PRESSED);")
    code.extend(f"press({key});" for key in keys)
    if control:
        code.append("send(Clutter.KEY_Control_L, Clutter.KeyState.RELEASED);")
    code.append("'ok'")
    shell_eval("\n".join(code))


def type_text(text):
    press(*[str(ord(letter)) for letter in text])


def focus_window():
    """A headless session starts in the overview, with nothing focused until a window is activated."""
    shell_eval(
        "Main.overview.hide();"
        "global.get_window_actors().map(a => a.meta_window).filter(w => w.get_title() === 'Dayweave')"
        ".forEach(w => w.activate(global.get_current_time()));"
        "'ok'"
    )
    has_focus = "global.display.focus_window ? global.display.focus_window.get_title() : ''"
    if wait_for(lambda: shell_eval(has_focus) == "Dayweave", 5) is False:
        raise RuntimeError("the window did not take focus")


def capture(name):
    time.sleep(1.8)
    path = OUTPUT / name
    reply = subprocess.run(
        ["gdbus", "call", "--session", "-d", "org.gnome.Shell", "-o", "/org/gnome/Shell/Screenshot",
         "-m", "org.gnome.Shell.Screenshot.ScreenshotWindow", "true", "false", "false", str(path)],
        capture_output=True, text=True, check=True,
    ).stdout
    if not reply.startswith("(true") or path.stat().st_size == 0:
        raise RuntimeError(f"GNOME refused the screenshot {name}: {reply}")
    print(f"captured {name} ({path.stat().st_size // 1024} KB)", flush=True)


def tour(theme):
    env = dict(os.environ, DAYWEAVE_TEST_THEME=theme)
    log = open(Path(os.environ["DAYWEAVE_TEST_WORK"]) / f"app-{theme}.log", "w")
    app = subprocess.Popen([APP], stdout=log, stderr=log, env=env)
    if not wait_for(windows, 25):
        raise RuntimeError("the window did not open")
    time.sleep(3)
    focus_window()
    # Registers the keyboard before anything depends on it.
    press("Clutter.KEY_Shift_L")
    time.sleep(1.5)

    # Ctrl+1 to Ctrl+7 follow the sidebar's order.
    pages = {"1": "overview", "2": "tasks", "3": "projects", "4": "habits", "5": "calendar", "6": "analytics", "7": "focus"}
    for key, page in pages.items():
        press(f"Clutter.KEY_{key}", control=True)
        capture(f"{page}-{theme}.png")

    press("Clutter.KEY_comma", control=True)
    capture(f"settings-{theme}.png")

    # J moves to the first task, and Enter opens it.
    press("Clutter.KEY_2", control=True)
    time.sleep(1)
    press("Clutter.KEY_j", "Clutter.KEY_j", "Clutter.KEY_j", "Clutter.KEY_Return")
    capture(f"task-details-{theme}.png")
    press("Clutter.KEY_Escape")

    press("Clutter.KEY_k", control=True)
    time.sleep(0.6)
    type_text("re")
    capture(f"command-palette-{theme}.png")
    press("Clutter.KEY_Escape")

    app.terminate()
    app.wait(timeout=5)


def main():
    OUTPUT.mkdir(parents=True, exist_ok=True)
    bridge = subprocess.Popen([BRIDGE, str(DATA), str(PORT)], stdout=subprocess.DEVNULL)
    wait_for_bridge()
    thesis, site, generator = seed_people_data()
    bridge.terminate()
    bridge.wait()
    backdate_history(thesis, site, generator)

    for theme in ("dark", "light"):
        subprocess.run(["gsettings", "set", "org.gnome.desktop.interface", "color-scheme",
                        "prefer-dark" if theme == "dark" else "default"], check=True)
        subprocess.run(["gsettings", "set", "org.gnome.desktop.interface", "gtk-theme",
                        "Yaru-dark" if theme == "dark" else "Yaru"], check=True)
        connection = sqlite3.connect(DATABASE)
        connection.execute(
            "INSERT INTO app_settings (key, value) VALUES ('theme', ?) "
            "ON CONFLICT (key) DO UPDATE SET value = excluded.value", (f'"{theme}"',))
        connection.commit()
        connection.close()
        tour(theme)
    shutil.rmtree(DATA, ignore_errors=True)


main()

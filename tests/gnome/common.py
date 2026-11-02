"""Helpers shared by the checks that run inside tests/gnome/session.sh."""

import json
import subprocess
import time
import urllib.request
from datetime import datetime, timedelta

PORT = 1499


def shell_eval(code):
    out = subprocess.run(
        ["gdbus", "call", "--session", "-d", "org.gnome.Shell", "-o", "/org/gnome/Shell",
         "-m", "org.gnome.Shell.Eval", code],
        capture_output=True, text=True, check=True,
    ).stdout.strip()
    if not out.startswith("(true"):
        raise RuntimeError(f"Shell.Eval failed: {out}")
    # The reply is a GVariant tuple: (true, 'json').
    text = out[out.index(",") + 1:].strip().rstrip(")").strip()
    if text.startswith(("'", '"')):
        text = text[1:-1].replace("\\'", "'").replace('\\"', '"').replace("\\\\", "\\")
    value = json.loads(text) if text else None
    # Shell.Eval serializes its result, so a string here may be JSON produced by the expression itself.
    if isinstance(value, str):
        try:
            return json.loads(value)
        except ValueError:
            return value
    return value


def windows():
    code = (
        "global.get_window_actors().map(a => a.meta_window)"
        ".filter(w => w.get_window_type() === 0 && !w.is_hidden())"
        ".map(w => ({ title: w.get_title(), app: w.get_wm_class() }))"
    )
    return shell_eval(f"JSON.stringify({code})") or []


def wait_for(predicate, timeout=20):
    end = time.time() + timeout
    while time.time() < end:
        value = predicate()
        if value:
            return value
        time.sleep(0.25)
    return predicate()


def rpc(method, params=None):
    request = urllib.request.Request(
        f"http://127.0.0.1:{PORT}", data=json.dumps({"method": method, "params": params or {}}).encode(), method="POST"
    )
    with urllib.request.urlopen(request) as response:
        return json.load(response)


def wait_for_bridge():
    for _ in range(100):
        try:
            rpc("app.info")
            return
        except OSError:
            time.sleep(0.1)
    raise RuntimeError("the bridge did not start")


def day(offset):
    return (datetime.now() + timedelta(days=offset)).strftime("%Y-%m-%d")

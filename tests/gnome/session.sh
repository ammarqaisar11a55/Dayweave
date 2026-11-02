#!/usr/bin/env bash
# Starts a disposable headless GNOME Shell, then runs the given command inside it. Nothing touches
# the real desktop: the session has its own D-Bus, Wayland socket, home and settings.
#
# usage: tests/gnome/session.sh command [arguments...]
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
helper=dayweave-test-helper@ammarqaisar11a55.github.io

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export HOME=$work/home XDG_DATA_HOME=$work/data XDG_CONFIG_HOME=$work/config XDG_CACHE_HOME=$work/cache
export XDG_RUNTIME_DIR=$work/run DAYWEAVE_TEST_WORK=$work
mkdir -p "$HOME" "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME"
mkdir -m 700 "$XDG_RUNTIME_DIR"

mkdir -p "$XDG_DATA_HOME/gnome-shell/extensions/$helper"
cp "$root"/tests/gnome/helper/* "$XDG_DATA_HOME/gnome-shell/extensions/$helper/"

unset DISPLAY WAYLAND_DISPLAY WAYLAND_SOCKET SESSION_MANAGER
export XDG_SESSION_TYPE=wayland XDG_CURRENT_DESKTOP=GNOME GDK_BACKEND=wayland
export GNOME_TEST_HELPER=$helper

exec dbus-run-session -- bash -c '
  set -u
  gsettings set org.gnome.shell enabled-extensions "[\"$GNOME_TEST_HELPER\"]"
  gsettings set org.gnome.shell disable-user-extensions false
  gsettings set org.gnome.shell welcome-dialog-last-shown-version "999"

  gnome-shell --headless --wayland --no-x11 --wayland-display=wayland-dayweave \
    --virtual-monitor 1600x1000 >"$DAYWEAVE_TEST_WORK/shell.log" 2>&1 &
  shell=$!
  export WAYLAND_DISPLAY=wayland-dayweave

  for _ in $(seq 1 100); do
    if gdbus call --session -d org.gnome.Shell -o /org/gnome/Shell \
        -m org.freedesktop.DBus.Properties.Get org.gnome.Shell ShellVersion >/dev/null 2>&1; then
      break
    fi
    sleep 0.2
  done
  sleep 1.5

  "$@"
  status=$?
  kill "$shell" 2>/dev/null
  wait "$shell" 2>/dev/null
  exit $status
' dayweave-session "$@"

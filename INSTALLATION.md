# Installation

Dayweave is built for Ubuntu 24.04 and newer.

## Snap

Dayweave is on the Snap Store's beta channel:

```bash
sudo snap install dayweave --beta
```

The stable channel will follow once the snap has been tried on real desktops.

To install a `.snap` file you built or downloaded from a release:

```bash
sudo snap install ./dayweave_0.1.0_amd64.snap --dangerous
```

Strict confinement means the snap can't reach the network, and can't read your home folder.
Backups and exports go wherever you choose in the file dialog, and only that file is shared with
Dayweave.

## .deb

Download the `.deb` from [GitHub Releases](https://github.com/ammarqaisar11a55/Dayweave/releases):

```bash
sudo apt install ./Dayweave_0.1.0_amd64.deb
```

## AppImage

Download the AppImage from the same page, then:

```bash
chmod +x Dayweave_0.1.0_amd64.AppImage
```

```bash
./Dayweave_0.1.0_amd64.AppImage
```

## Where your data is

| Package | Location |
| --- | --- |
| `.deb`, AppImage | `~/.local/share/io.github.ammarqaisar11a55.dayweave/` |
| Snap | `~/snap/dayweave/current/.local/share/io.github.ammarqaisar11a55.dayweave/` |

Settings shows the exact folder. Upgrades keep your data. The snap also keeps a copy per revision,
so `snap revert` returns to the matching data.

Your data doesn't move between packages automatically. To switch, save a full backup in the old
package from Settings, then restore it in the new one.

## The tray icon

Closing the window leaves Dayweave in the tray so reminders keep working. Ubuntu shows tray icons
out of the box. On a plain GNOME desktop without an AppIndicator extension there is no icon, but
Dayweave still runs in the background, and launching it again reopens the window.

## Uninstalling

Removing the `.deb` or deleting the AppImage leaves your data in place. Removing the snap removes
its data, and snapd keeps an automatic snapshot for a limited time. Save a backup first if you may
want your data again.

//! The "start at login" entry. Writing the file directly works in both packages: a snap's config
//! directory is the one `snap userd` reads, given the `autostart` line in snapcraft.yaml.

use std::fs;
use std::io;
use std::path::PathBuf;

const FILE: &str = "dayweave.desktop";

fn entry_path() -> Option<PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(config.join("autostart").join(FILE))
}

fn command() -> String {
    if std::env::var_os("SNAP_NAME").is_some() {
        return "/snap/bin/dayweave".into();
    }
    if let Some(image) = std::env::var_os("APPIMAGE") {
        return image.to_string_lossy().into_owned();
    }
    std::env::current_exe()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "dayweave".into())
}

fn quoted(text: &str) -> String {
    if text.contains(char::is_whitespace) {
        format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        text.to_string()
    }
}

pub fn set(enabled: bool) -> io::Result<()> {
    let path = entry_path().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no home directory"))?;
    if !enabled {
        return match fs::remove_file(path) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        };
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        format!(
            "[Desktop Entry]\nType=Application\nName=Dayweave\nComment=Delivers your reminders\n\
             Exec={} --background\nIcon=dayweave\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
            quoted(&command())
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_is_written_and_removed_and_starts_in_the_background() {
        let directory = std::env::temp_dir().join(format!("dayweave-autostart-{}", std::process::id()));
        std::env::set_var("XDG_CONFIG_HOME", &directory);
        let entry = directory.join("autostart").join(FILE);

        set(true).unwrap();
        let text = fs::read_to_string(&entry).unwrap();
        assert!(text.lines().any(|line| line.starts_with("Exec=") && line.ends_with(" --background")));

        set(false).unwrap();
        assert!(!entry.exists());
        set(false).expect("turning it off twice is harmless");

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_path_with_spaces_is_quoted() {
        assert_eq!(quoted("/usr/bin/dayweave"), "/usr/bin/dayweave");
        assert_eq!(quoted("/home/a b/Dayweave.AppImage"), "\"/home/a b/Dayweave.AppImage\"");
    }
}

//! Recognition and management of Progressive Web Apps (PWAs) installed via
//! Chromium-based browsers (Google Chrome, Microsoft Edge, Brave, etc.).
//!
//! PWAs run in their own application windows with dedicated AppUserModelIDs
//! (typically containing `_crx_`), distinct window titles, and individual icons.

use std::{
    env, fs,
    path::{Path, PathBuf},
};
use windows::{
    core::HSTRING,
    Win32::{
        Foundation::HWND,
        Storage::EnhancedStorage::PKEY_AppUserModel_ID,
        UI::Shell::{
            PropertiesSystem::{GPS_DEFAULT, IPropertyStore, SHGetPropertyStoreForWindow, SHGetPropertyStoreFromParsingName},
        },
    },
};

pub const PWA_PREFIX: &str = "pwa:";

/// Checks if an identifier represents an installed web application.
#[must_use]
pub fn is_pwa(id: &str) -> bool {
    id.starts_with(PWA_PREFIX)
}

/// Returns the raw AppUserModelID of a PWA from its `pwa:` prefixed identifier.
#[must_use]
pub fn pwa_aumid(id: &str) -> Option<&str> {
    id.strip_prefix(PWA_PREFIX)
}

/// Detects if an application window is a standalone PWA rather than a standard browser window.
///
/// Chromium sets a custom `PKEY_AppUserModel_ID` on PWA top-level windows.
/// The AUMID format is typically `<Browser>._crx_<ExtensionId>` (for example
/// `Chrome._crx_agimnkijcaahngcdmfeangaknmldooml` or `MSEdge._crx_...`).
pub fn detect_pwa(hwnd: HWND, image: &str) -> Option<String> {
    let exe = image.rsplit('\\').next().unwrap_or("").to_lowercase();
    if !matches!(
        exe.as_str(),
        "chrome.exe" | "msedge.exe" | "brave.exe" | "chromium.exe" | "vivaldi.exe" | "opera.exe"
    ) {
        return None;
    }

    let aumid = window_aumid(hwnd)?;
    let lower = aumid.to_lowercase();

    if lower.contains("._crx_")
        || lower.contains("_crx_")
        || lower.contains(".pwa.")
        || lower.contains("!app")
        || lower.starts_with("app-")
        || is_custom_pwa_aumid(&lower)
    {
        Some(format!("{PWA_PREFIX}{aumid}"))
    } else {
        None
    }
}

fn window_aumid(hwnd: HWND) -> Option<String> {
    unsafe {
        let store: IPropertyStore = SHGetPropertyStoreForWindow(hwnd).ok()?;
        let value = store.GetValue(&PKEY_AppUserModel_ID).ok()?;
        let str_val = value.to_string();
        (!str_val.is_empty()).then_some(str_val)
    }
}

fn is_custom_pwa_aumid(lower: &str) -> bool {
    !matches!(
        lower,
        "chrome"
            | "google.chrome"
            | "msedge"
            | "microsoft.edge"
            | "brave"
            | "bravebrowser"
            | "bravesoftware.brave"
            | "chromium"
            | "vivaldi"
    ) && lower.contains('.')
}

/// Extracts a friendly display name for a PWA.
#[must_use]
pub fn pwa_display_name(pwa_id: &str) -> String {
    if let Some(shortcut) = find_pwa_shortcut(pwa_id) {
        if let Some(stem) = shortcut.file_stem().and_then(|s| s.to_str()) {
            return stem.to_owned();
        }
    }
    if let Some(aumid) = pwa_aumid(pwa_id) {
        if let Some((_, app_id)) = aumid.split_once("_crx_") {
            return format!("Web App ({})", &app_id[..8.min(app_id.len())]);
        }
        return aumid.to_owned();
    }
    "Web App".to_owned()
}

/// Searches the system for the shortcut (.lnk) corresponding to a PWA identifier.
#[must_use]
pub fn find_pwa_shortcut(pwa_id: &str) -> Option<PathBuf> {
    let aumid = pwa_aumid(pwa_id)?;
    let crx_id = aumid.split("_crx_").nth(1).unwrap_or(aumid);

    let appdata = env::var_os("APPDATA").map(PathBuf::from);
    let localappdata = env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let userprofile = env::var_os("USERPROFILE").map(PathBuf::from);

    let mut search_dirs: Vec<PathBuf> = Vec::new();

    if let Some(ref appdata) = appdata {
        let start_programs = appdata.join(r"Microsoft\Windows\Start Menu\Programs");
        search_dirs.push(start_programs.join("Chrome Apps"));
        search_dirs.push(start_programs.join("Edge Apps"));
        search_dirs.push(start_programs.join("Brave Apps"));
        search_dirs.push(start_programs.join(r"Brave Software\Brave-Browser Apps"));
        search_dirs.push(start_programs);
        search_dirs.push(appdata.join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar"));
        search_dirs.push(appdata.join(r"Flote\Pins"));
        search_dirs.push(appdata.join(r"Muelle\Pins"));
    }
    if let Some(ref local) = localappdata {
        search_dirs.push(local.join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Some(ref home) = userprofile {
        search_dirs.push(home.join("Desktop"));
    }

    for dir in search_dirs {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("lnk")) {
                    if shortcut_matches_pwa(&path, aumid, crx_id) {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

fn shortcut_matches_pwa(link: &Path, aumid: &str, crx_id: &str) -> bool {
    // 1. Check shortcut arguments for --app-id=<crx_id>
    if let Some((_, arguments)) = crate::pins::link_target_and_arguments(link) {
        if !arguments.is_empty() {
            let lower_args = arguments.to_lowercase();
            let target_arg = format!("--app-id={}", crx_id.to_lowercase());
            if lower_args.contains(&target_arg) {
                return true;
            }
        }
    }

    // 2. Check shortcut property store for PKEY_AppUserModel_ID
    unsafe {
        if let Ok(store) = SHGetPropertyStoreFromParsingName::<IPropertyStore>(
            &HSTRING::from(link.as_os_str()),
            None,
            GPS_DEFAULT,
        ) {
            if let Ok(prop) = store.GetValue(&PKEY_AppUserModel_ID) {
                let shortcut_aumid = prop.to_string();
                if shortcut_aumid.eq_ignore_ascii_case(aumid) {
                    return true;
                }
            }
        }
    }

    false
}

/// Identifies if a shortcut is for an installed PWA and returns its `pwa:<aumid>` ID.
pub fn pwa_id_from_shortcut(link: &Path, target: &str, arguments: &str) -> Option<String> {
    let exe = target.rsplit('\\').next().unwrap_or("").to_lowercase();
    if !matches!(
        exe.as_str(),
        "chrome.exe" | "msedge.exe" | "brave.exe" | "chromium.exe" | "vivaldi.exe"
    ) {
        return None;
    }

    // Check arguments for --app-id=<id>
    if let Some(pos) = arguments.find("--app-id=") {
        let after = &arguments[pos + 9..];
        let app_id = after.split_whitespace().next().unwrap_or("").trim_matches('"');
        if !app_id.is_empty() {
            let prefix = if exe.contains("chrome") {
                "Chrome"
            } else if exe.contains("edge") {
                "MSEdge"
            } else if exe.contains("brave") {
                "Brave"
            } else {
                "Chromium"
            };
            return Some(format!("{PWA_PREFIX}{}._crx_{}", prefix, app_id));
        }
    }

    // Check shortcut property store for PKEY_AppUserModel_ID
    unsafe {
        if let Ok(store) = SHGetPropertyStoreFromParsingName::<IPropertyStore>(
            &HSTRING::from(link.as_os_str()),
            None,
            GPS_DEFAULT,
        ) {
            if let Ok(prop) = store.GetValue(&PKEY_AppUserModel_ID) {
                let aumid = prop.to_string();
                if aumid.contains("._crx_") || aumid.contains("_crx_") || aumid.contains(".pwa.") {
                    return Some(format!("{PWA_PREFIX}{aumid}"));
                }
            }
        }
    }

    None
}

/// Looks up if the user configured a custom icon for `app_id` in `config.json`.
#[must_use]
pub fn custom_icon_path(app_id: &str) -> Option<String> {
    let appdata = PathBuf::from(env::var_os("APPDATA")?);
    let flote_path = appdata.join(r"Flote\config.json");
    let config_path = if flote_path.exists() {
        flote_path
    } else {
        appdata.join(r"Muelle\config.json")
    };
    let content = fs::read_to_string(config_path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    let custom_icons = value.get("customIcons")?.as_object()?;
    let icon_file = custom_icons.get(app_id)?.as_str()?;
    let path = Path::new(icon_file);
    if path.is_file() {
        Some(icon_file.to_owned())
    } else {
        None
    }
}

use std::sync::Arc;

use fetch_core::{ApplicationSettings, FetchError, SettingsOperations};
use fetch_storage::Storage;

use crate::tray::TrayState;

pub trait StartupRegistration: Send + Sync {
    fn set_enabled(&self, enabled: bool) -> Result<(), String>;
}

#[cfg(target_os = "linux")]
pub struct SystemStartupRegistration {
    desktop_file: std::path::PathBuf,
    icon_file: std::path::PathBuf,
    executable: std::path::PathBuf,
}

#[cfg(target_os = "windows")]
pub struct SystemStartupRegistration {
    executable: std::path::PathBuf,
}

#[cfg(target_os = "macos")]
pub struct SystemStartupRegistration {
    plist_file: std::path::PathBuf,
    executable: std::path::PathBuf,
}

impl SystemStartupRegistration {
    #[cfg(target_os = "linux")]
    pub fn new() -> Result<Self, String> {
        let executable = std::env::current_exe()
            .map_err(|error| format!("could not locate the Fetch executable: {error}"))?;
        let base_dirs = directories::BaseDirs::new()
            .ok_or_else(|| "could not locate the user configuration directory".to_owned())?;
        let config = base_dirs
            .config_dir()
            .join("autostart")
            .join("fetch.desktop");
        let icon_file = base_dirs
            .data_dir()
            .join("icons/hicolor/256x256/apps/fetch.png");
        Ok(Self {
            desktop_file: config,
            icon_file,
            executable,
        })
    }

    #[cfg(target_os = "windows")]
    pub fn new() -> Result<Self, String> {
        let executable = std::env::current_exe()
            .map_err(|error| format!("could not locate the Fetch executable: {error}"))?;
        Ok(Self { executable })
    }

    #[cfg(target_os = "macos")]
    pub fn new() -> Result<Self, String> {
        let executable = std::env::current_exe()
            .map_err(|error| format!("could not locate the Fetch executable: {error}"))?;
        let plist_file = directories::BaseDirs::new()
            .ok_or_else(|| "could not locate the user home directory".to_owned())?
            .home_dir()
            .join("Library/LaunchAgents/dev.fetch.Fetch.plist");
        Ok(Self {
            plist_file,
            executable,
        })
    }
}

impl StartupRegistration for SystemStartupRegistration {
    #[cfg(target_os = "linux")]
    fn set_enabled(&self, enabled: bool) -> Result<(), String> {
        if enabled {
            let parent = self
                .desktop_file
                .parent()
                .ok_or_else(|| "the XDG autostart path has no parent".to_owned())?;
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            let icon_parent = self
                .icon_file
                .parent()
                .ok_or_else(|| "the XDG icon path has no parent".to_owned())?;
            std::fs::create_dir_all(icon_parent).map_err(|error| error.to_string())?;
            std::fs::write(&self.icon_file, include_bytes!("../assets/fetch.png"))
                .map_err(|error| error.to_string())?;
            let entry = linux_desktop_entry(&self.executable)?;
            if let Err(error) = std::fs::write(&self.desktop_file, entry) {
                let _ = std::fs::remove_file(&self.icon_file);
                return Err(error.to_string());
            }
            Ok(())
        } else {
            remove_if_present(&self.desktop_file)?;
            remove_if_present(&self.icon_file)
        }
    }

    #[cfg(target_os = "windows")]
    fn set_enabled(&self, enabled: bool) -> Result<(), String> {
        const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
        let key = windows_registry::CURRENT_USER
            .create(RUN_KEY)
            .map_err(|error| error.to_string())?;
        if enabled {
            key.set_string("Fetch", windows_start_command(&self.executable)?)
                .map_err(|error| error.to_string())
        } else {
            let exists = key
                .values()
                .map_err(|error| error.to_string())?
                .any(|(name, _)| name == "Fetch");
            if exists {
                key.remove_value("Fetch").map_err(|error| error.to_string())
            } else {
                Ok(())
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn set_enabled(&self, enabled: bool) -> Result<(), String> {
        if enabled {
            let parent = self
                .plist_file
                .parent()
                .ok_or_else(|| "the LaunchAgent path has no parent".to_owned())?;
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            let plist = macos_launch_agent(&self.executable)?;
            std::fs::write(&self.plist_file, plist).map_err(|error| error.to_string())
        } else {
            match std::fs::remove_file(&self.plist_file) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error.to_string()),
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_desktop_entry(executable: &std::path::Path) -> Result<String, String> {
    let executable = executable
        .to_str()
        .ok_or_else(|| "the Fetch executable path is not valid Unicode".to_owned())?;
    let escaped = executable
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$");
    Ok(format!(
        "[Desktop Entry]\nType=Application\nVersion=1.0\nName=Fetch\nComment=Local media downloader\nExec=\"{escaped}\" --background\nIcon=fetch\nTerminal=false\nStartupNotify=false\n"
    ))
}

#[cfg(target_os = "linux")]
fn remove_if_present(path: &std::path::Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(any(target_os = "macos", test))]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(any(target_os = "windows", test))]
fn windows_start_command(executable: &std::path::Path) -> Result<String, String> {
    let executable = executable
        .to_str()
        .ok_or_else(|| "the Fetch executable path is not valid Unicode".to_owned())?;
    Ok(format!("\"{executable}\" --background"))
}

#[cfg(any(target_os = "macos", test))]
fn macos_launch_agent(executable: &std::path::Path) -> Result<String, String> {
    let executable = executable
        .to_str()
        .ok_or_else(|| "the Fetch executable path is not valid Unicode".to_owned())?;
    let executable = xml_escape(executable);
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>Label</key><string>dev.fetch.Fetch</string><key>ProgramArguments</key><array><string>{executable}</string><string>--background</string></array><key>RunAtLoad</key><true/></dict></plist>\n"
    ))
}

pub struct ManagedSettings {
    storage: Arc<Storage>,
    startup: Arc<dyn StartupRegistration>,
    tray_state: Arc<TrayState>,
}

impl ManagedSettings {
    pub fn new(
        storage: Arc<Storage>,
        startup: Arc<dyn StartupRegistration>,
        tray_state: Arc<TrayState>,
    ) -> Self {
        Self {
            storage,
            startup,
            tray_state,
        }
    }

    pub async fn reconcile_startup(&self, enabled: bool) -> Result<(), FetchError> {
        let startup = self.startup.clone();
        tokio::task::spawn_blocking(move || startup.set_enabled(enabled))
            .await
            .map_err(|error| FetchError::Internal(error.to_string()))?
            .map_err(|error| {
                FetchError::InvalidSettings(format!(
                    "could not update Start Fetch with system: {error}"
                ))
            })
    }
}

#[async_trait::async_trait]
impl SettingsOperations for ManagedSettings {
    async fn get_settings(&self) -> Result<ApplicationSettings, FetchError> {
        self.storage.get_settings().await
    }

    async fn put_settings(
        &self,
        settings: ApplicationSettings,
    ) -> Result<ApplicationSettings, FetchError> {
        settings.validate_basic()?;
        let previous = self.storage.get_settings().await?;
        self.reconcile_startup(settings.start_with_system).await?;
        if let Err(error) = self.storage.save_settings(&settings).await {
            let startup = self.startup.clone();
            let previous_enabled = previous.start_with_system;
            let _ =
                tokio::task::spawn_blocking(move || startup.set_enabled(previous_enabled)).await;
            return Err(FetchError::Internal(error.to_string()));
        }
        self.tray_state
            .set_download_directory(settings.download_directory.clone());
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct RecordingStartup(Mutex<Vec<bool>>);

    impl StartupRegistration for RecordingStartup {
        fn set_enabled(&self, enabled: bool) -> Result<(), String> {
            self.0.lock().unwrap().push(enabled);
            Ok(())
        }
    }

    #[tokio::test]
    async fn saving_settings_synchronizes_startup_registration() {
        let storage = Arc::new(
            Storage::open(std::path::Path::new(":memory:"))
                .await
                .unwrap(),
        );
        let initial = ApplicationSettings {
            bind_address: "127.0.0.1".parse().unwrap(),
            port: 8080,
            allowed_networks: vec![],
            download_directory: "downloads".into(),
            concurrent_downloads: 3,
            open_browser_on_start: true,
            start_with_system: false,
            ytdlp_auto_update: true,
            ytdlp_js_runtime: fetch_core::YtDlpJsRuntime::Auto,
        };
        storage.save_settings(&initial).await.unwrap();
        let startup = Arc::new(RecordingStartup::default());
        let tray_state = Arc::new(TrayState::new(
            "127.0.0.1:8080".parse().unwrap(),
            "downloads".into(),
        ));
        let settings = ManagedSettings::new(storage, startup.clone(), tray_state);

        settings
            .put_settings(ApplicationSettings {
                start_with_system: true,
                ..initial
            })
            .await
            .unwrap();

        assert_eq!(*startup.0.lock().unwrap(), [true]);
        assert!(settings.get_settings().await.unwrap().start_with_system);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_startup_entry_quotes_executable_paths() {
        let entry =
            linux_desktop_entry(std::path::Path::new("/home/Fetch App/fetch\"preview")).unwrap();
        assert!(entry.contains("Exec=\"/home/Fetch App/fetch\\\"preview\" --background"));
        assert!(entry.contains("Icon=fetch"));
        assert!(entry.contains("Terminal=false"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_registration_enables_and_disables_an_isolated_entry() {
        let directory = tempfile::tempdir().unwrap();
        let desktop_file = directory.path().join("autostart/fetch.desktop");
        let icon_file = directory
            .path()
            .join("icons/hicolor/256x256/apps/fetch.png");
        let registration = SystemStartupRegistration {
            desktop_file: desktop_file.clone(),
            icon_file: icon_file.clone(),
            executable: std::path::PathBuf::from("/opt/Fetch App/fetch"),
        };

        registration.set_enabled(true).unwrap();
        let entry = std::fs::read_to_string(&desktop_file).unwrap();
        assert!(entry.contains("Exec=\"/opt/Fetch App/fetch\" --background"));
        assert!(entry.contains("Icon=fetch"));
        assert_eq!(
            std::fs::read(&icon_file).unwrap(),
            include_bytes!("../assets/fetch.png")
        );
        registration.set_enabled(false).unwrap();
        assert!(!desktop_file.exists());
        assert!(!icon_file.exists());
    }

    #[test]
    fn native_startup_formats_quote_paths_and_fixed_background_argument() {
        assert_eq!(
            windows_start_command(std::path::Path::new("C:\\Fetch App\\fetch.exe")).unwrap(),
            "\"C:\\Fetch App\\fetch.exe\" --background"
        );
        let plist =
            macos_launch_agent(std::path::Path::new("/Applications/Fetch & Go/fetch")).unwrap();
        assert!(plist.contains("/Applications/Fetch &amp; Go/fetch"));
        assert!(plist.contains("<string>--background</string>"));
    }

    #[test]
    fn native_application_assets_are_well_formed() {
        let png = image::load_from_memory(include_bytes!("../assets/fetch.png")).unwrap();
        assert_eq!((png.width(), png.height()), (256, 256));

        let ico = include_bytes!("../assets/fetch.ico");
        assert_eq!(&ico[0..4], &[0, 0, 1, 0]);
        assert_eq!(u16::from_le_bytes([ico[4], ico[5]]), 7);

        let plist = include_str!("../assets/macos/Info.plist.in");
        assert!(plist.contains("<string>Fetch.icns</string>"));
        assert!(plist.contains("<string>io.fetch.downloader</string>"));
        assert_eq!(plist.matches("@VERSION@").count(), 2);
    }
}

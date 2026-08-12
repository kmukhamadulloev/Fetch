use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct TrayContext {
    pub state: Arc<TrayState>,
    pub shutdown: CancellationToken,
}

pub struct TrayState {
    address: RwLock<SocketAddr>,
    download_directory: RwLock<PathBuf>,
}

impl TrayState {
    pub fn new(address: SocketAddr, download_directory: PathBuf) -> Self {
        Self {
            address: RwLock::new(address),
            download_directory: RwLock::new(download_directory),
        }
    }

    pub fn set_address(&self, address: SocketAddr) {
        *self.address.write().expect("tray address lock poisoned") = address;
    }

    pub fn set_download_directory(&self, directory: PathBuf) {
        *self
            .download_directory
            .write()
            .expect("tray download-directory lock poisoned") = directory;
    }

    fn url(&self) -> String {
        let address = *self.address.read().expect("tray address lock poisoned");
        let ip = if address.ip().is_unspecified() {
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        } else {
            address.ip()
        };
        format!("http://{}/", SocketAddr::new(ip, address.port()))
    }

    fn download_directory(&self) -> PathBuf {
        self.download_directory
            .read()
            .expect("tray download-directory lock poisoned")
            .clone()
    }
}

fn open_fetch(state: &TrayState) {
    let url = state.url();
    if let Err(error) = webbrowser::open(&url) {
        tracing::warn!(%error, %url, "could not open Fetch from the tray");
    }
}

fn open_downloads(state: &TrayState) {
    let directory = state.download_directory();
    if let Err(error) = std::fs::create_dir_all(&directory) {
        tracing::warn!(%error, path = %directory.display(), "could not prepare the download directory");
        return;
    }
    if let Err(error) = open_directory(&directory) {
        tracing::warn!(%error, path = %directory.display(), "could not open the download directory");
    }
}

fn open_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("explorer.exe");
        command.arg(path);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("open");
        command.arg(path);
        command
    };
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let mut command = {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(path);
        command
    };
    command.spawn().map(|_| ())
}

#[cfg(target_os = "linux")]
pub async fn start(context: TrayContext) {
    use image::GenericImageView;
    use ksni::TrayMethods;

    struct FetchTray(TrayContext);

    impl ksni::Tray for FetchTray {
        fn id(&self) -> String {
            "fetch".into()
        }

        fn title(&self) -> String {
            "Fetch".into()
        }

        fn icon_pixmap(&self) -> Vec<ksni::Icon> {
            let image = image::load_from_memory_with_format(
                include_bytes!("../../../docs/assets/logo.png"),
                image::ImageFormat::Png,
            )
            .expect("embedded Fetch logo is valid PNG")
            .resize_exact(64, 64, image::imageops::FilterType::Lanczos3);
            let (width, height) = image.dimensions();
            let mut data = image.into_rgba8().into_raw();
            for pixel in data.chunks_exact_mut(4) {
                pixel.rotate_right(1);
            }
            vec![ksni::Icon {
                width: width as i32,
                height: height as i32,
                data,
            }]
        }

        fn activate(&mut self, _x: i32, _y: i32) {
            open_fetch(&self.0.state);
        }

        fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
            use ksni::menu::*;
            vec![
                StandardItem {
                    label: "Open Fetch".into(),
                    activate: Box::new(|tray: &mut Self| open_fetch(&tray.0.state)),
                    ..Default::default()
                }
                .into(),
                StandardItem {
                    label: "Open downloads folder".into(),
                    activate: Box::new(|tray: &mut Self| open_downloads(&tray.0.state)),
                    ..Default::default()
                }
                .into(),
                ksni::MenuItem::Separator,
                StandardItem {
                    label: "Quit Fetch".into(),
                    activate: Box::new(|tray: &mut Self| tray.0.shutdown.cancel()),
                    ..Default::default()
                }
                .into(),
            ]
        }
    }

    match FetchTray(context.clone()).spawn().await {
        Ok(handle) => {
            tracing::info!("Fetch system tray ready");
            tokio::spawn(async move {
                context.shutdown.cancelled().await;
                let _ = handle.shutdown().await;
            });
        }
        Err(error) => {
            tracing::warn!(%error, "system tray is unavailable; Fetch remains accessible through the browser and terminal")
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
mod desktop {
    use super::*;
    use tray_icon::{
        TrayIcon, TrayIconBuilder, TrayIconEvent,
        menu::{Menu, MenuEvent, MenuItem},
    };
    use winit::{
        application::ApplicationHandler,
        event_loop::{ActiveEventLoop, EventLoop},
        window::WindowId,
    };

    enum UserEvent {
        Ready(TrayContext),
        Menu(MenuEvent),
        Tray(TrayIconEvent),
        Stopped(Option<String>),
    }

    struct TrayApplication {
        context: Option<TrayContext>,
        tray: Option<TrayIcon>,
        open: Option<MenuItem>,
        downloads: Option<MenuItem>,
        quit: Option<MenuItem>,
        service_error: Option<String>,
    }

    impl TrayApplication {
        fn new() -> Self {
            Self {
                context: None,
                tray: None,
                open: None,
                downloads: None,
                quit: None,
                service_error: None,
            }
        }

        fn install(&mut self, context: TrayContext) -> Result<(), String> {
            let menu = Menu::new();
            let open = MenuItem::new("Open Fetch", true, None);
            let downloads = MenuItem::new("Open downloads folder", true, None);
            let quit = MenuItem::new("Quit Fetch", true, None);
            menu.append_items(&[
                &open,
                &downloads,
                &tray_icon::menu::PredefinedMenuItem::separator(),
                &quit,
            ])
            .map_err(|error| error.to_string())?;
            let image = image::load_from_memory_with_format(
                include_bytes!("../../../docs/assets/logo.png"),
                image::ImageFormat::Png,
            )
            .map_err(|error| error.to_string())?
            .resize_exact(64, 64, image::imageops::FilterType::Lanczos3)
            .into_rgba8();
            let (width, height) = image.dimensions();
            let icon = tray_icon::Icon::from_rgba(image.into_raw(), width, height)
                .map_err(|error| error.to_string())?;
            self.tray = Some(
                TrayIconBuilder::new()
                    .with_tooltip("Fetch")
                    .with_icon(icon)
                    .with_menu(Box::new(menu))
                    .build()
                    .map_err(|error| error.to_string())?,
            );
            self.context = Some(context);
            self.open = Some(open);
            self.downloads = Some(downloads);
            self.quit = Some(quit);
            tracing::info!("Fetch system tray ready");
            Ok(())
        }
    }

    impl ApplicationHandler<UserEvent> for TrayApplication {
        fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

        fn window_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            _event: winit::event::WindowEvent,
        ) {
        }

        fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
            match event {
                UserEvent::Ready(context) => {
                    if let Err(error) = self.install(context) {
                        tracing::warn!(%error, "system tray is unavailable; Fetch remains accessible through the browser and terminal");
                    }
                }
                UserEvent::Menu(event) => {
                    let Some(context) = self.context.as_ref() else {
                        return;
                    };
                    if self.open.as_ref().is_some_and(|item| event.id == item.id()) {
                        open_fetch(&context.state);
                    } else if self
                        .downloads
                        .as_ref()
                        .is_some_and(|item| event.id == item.id())
                    {
                        open_downloads(&context.state);
                    } else if self.quit.as_ref().is_some_and(|item| event.id == item.id()) {
                        context.shutdown.cancel();
                    }
                }
                UserEvent::Tray(TrayIconEvent::Click {
                    button: tray_icon::MouseButton::Left,
                    button_state: tray_icon::MouseButtonState::Up,
                    ..
                }) => {
                    if let Some(context) = self.context.as_ref() {
                        open_fetch(&context.state);
                    }
                }
                UserEvent::Tray(_) => {}
                UserEvent::Stopped(error) => {
                    self.service_error = error;
                    event_loop.exit();
                }
            }
        }
    }

    pub fn run(
        service: impl FnOnce(Box<dyn FnOnce(TrayContext) + Send>) -> Result<(), String> + Send + 'static,
    ) -> anyhow::Result<()> {
        let mut event_loop_builder = EventLoop::<UserEvent>::with_user_event();
        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
            event_loop_builder.with_activation_policy(ActivationPolicy::Accessory);
        }
        let event_loop = event_loop_builder.build()?;
        let proxy = event_loop.create_proxy();
        MenuEvent::set_event_handler(Some(move |event| {
            let _ = proxy.send_event(UserEvent::Menu(event));
        }));
        let proxy = event_loop.create_proxy();
        TrayIconEvent::set_event_handler(Some(move |event| {
            let _ = proxy.send_event(UserEvent::Tray(event));
        }));
        let proxy = event_loop.create_proxy();
        std::thread::spawn(move || {
            let ready_proxy = proxy.clone();
            let result = service(Box::new(move |context| {
                let _ = ready_proxy.send_event(UserEvent::Ready(context));
            }));
            let _ = proxy.send_event(UserEvent::Stopped(result.err()));
        });
        let mut application = TrayApplication::new();
        event_loop.run_app(&mut application)?;
        if let Some(error) = application.service_error {
            return Err(anyhow::anyhow!(error));
        }
        Ok(())
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub use desktop::run;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_url_uses_loopback_for_wildcard_bind() {
        let state = TrayState::new("0.0.0.0:9080".parse().unwrap(), PathBuf::from("downloads"));
        assert_eq!(state.url(), "http://127.0.0.1:9080/");
        state.set_address("[::1]:9090".parse().unwrap());
        assert_eq!(state.url(), "http://[::1]:9090/");
    }
}

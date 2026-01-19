use std::sync::mpsc::{self, Receiver, Sender};
use image::GenericImageView;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, TrayIconBuilder,
};

#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
};

const ICON: &[u8] = include_bytes!("../data/icon.ico");

pub enum TrayCommand {
    Exit,
}

enum TrayMessage {
    SetTooltip(String),
}

pub struct Tray {
    cmd_receiver: Receiver<TrayCommand>,
    msg_sender: Sender<TrayMessage>,
}

impl Tray {
    pub fn new() -> Option<Self> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<TrayCommand>();
        let (msg_tx, msg_rx) = mpsc::channel::<TrayMessage>();

        std::thread::spawn(move || {
            let menu = Menu::new();
            let exit_item = MenuItem::new("종료", true, None);
            let exit_id = exit_item.id().clone();
            menu.append(&exit_item).ok();

            let icon = create_icon();

            let tray = TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_tooltip("무작위 총력전: 아수라장 어드바이저")
                .with_icon(icon)
                .build()
                .expect("Failed to create tray");

            loop {
                #[cfg(windows)]
                unsafe {
                    let mut msg = MSG::default();
                    while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }

                while let Ok(msg) = msg_rx.try_recv() {
                    match msg {
                        TrayMessage::SetTooltip(tooltip) => {
                            let _ = tray.set_tooltip(Some(&tooltip));
                        }
                    }
                }

                if let Ok(event) = MenuEvent::receiver().try_recv() {
                    if event.id == exit_id {
                        let _ = cmd_tx.send(TrayCommand::Exit);
                        break;
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });

        Some(Self {
            cmd_receiver: cmd_rx,
            msg_sender: msg_tx,
        })
    }

    pub fn poll(&self) -> Option<TrayCommand> {
        self.cmd_receiver.try_recv().ok()
    }

    pub fn set_tooltip(&self, tooltip: &str) {
        let _ = self.msg_sender.send(TrayMessage::SetTooltip(tooltip.to_string()));
    }
}

fn create_icon() -> Icon {
    let img = image::load_from_memory(ICON).expect("Failed to load icon");
    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8().into_raw();
    Icon::from_rgba(rgba, width, height).expect("Failed to create icon")
}

use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::event_loop::EventLoopProxy;
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

use crate::capture::get_lol_window;

#[derive(Clone, Debug, serde::Serialize)]
pub struct AugmentDisplay {
    pub name: String,
    pub tier: String,
    pub popularity: String,
    pub games: i32,
}

#[derive(Debug, Clone)]
pub enum OverlayEvent {
    Update { index: usize, data: Option<AugmentDisplay>, x: i32, y: i32 },
    Show(usize),
    Hide(usize),
    HideAll,
}

const CARD_HTML: &str = include_str!("../data/overlay.html");

pub struct Overlay {
    proxy: EventLoopProxy<OverlayEvent>,
}

impl Overlay {
    pub fn new() -> Option<Self> {
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            use tao::event::{Event, WindowEvent};
            use tao::event_loop::{ControlFlow, EventLoopBuilder};
            use tao::platform::windows::EventLoopBuilderExtWindows;

            let event_loop = EventLoopBuilder::<OverlayEvent>::with_user_event()
                .with_any_thread(true)
                .build();
            let proxy = event_loop.create_proxy();
            let _ = tx.send(proxy);

            let mut windows = Vec::with_capacity(3);
            let mut webviews = Vec::with_capacity(3);

            for _ in 0..3 {
                let window = WindowBuilder::new()
                    .with_title("Augment Overlay")
                    .with_position(PhysicalPosition::new(0, 0))
                    .with_inner_size(PhysicalSize::new(200u32, 150u32))
                    .with_decorations(false)
                    .with_transparent(true)
                    .with_always_on_top(true)
                    .with_resizable(false)
                    .with_visible(false)
                    .build(&event_loop)
                    .expect("Failed to create window");

                window.set_ignore_cursor_events(true).ok();

                let webview = WebViewBuilder::new()
                    .with_transparent(true)
                    .with_html(CARD_HTML)
                    .build(&window)
                    .expect("Failed to create webview");

                windows.push(window);
                webviews.push(webview);
            }

            event_loop.run(move |event, _, control_flow| {
                *control_flow = ControlFlow::Wait;

                match event {
                    Event::UserEvent(overlay_event) => match overlay_event {
                        OverlayEvent::Update { index, data, x, y } => {
                            if index < 3 {
                                windows[index].set_outer_position(PhysicalPosition::new(x, y));
                                if let Some(aug) = data {
                                    let json = serde_json::to_string(&aug).unwrap_or_default();
                                    let script = format!("window.updateCard('{}');", json.replace('\'', "\\'"));
                                    let _ = webviews[index].evaluate_script(&script);
                                }
                            }
                        }
                        OverlayEvent::Show(index) => {
                            if index < 3 {
                                windows[index].set_visible(true);
                            }
                        }
                        OverlayEvent::Hide(index) => {
                            if index < 3 {
                                windows[index].set_visible(false);
                            }
                        }
                        OverlayEvent::HideAll => {
                            for w in &windows {
                                w.set_visible(false);
                            }
                        }
                    },
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        ..
                    } => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            });
        });

        let proxy = rx.recv().ok()?;
        Some(Self { proxy })
    }

    pub fn update(&self, index: usize, augment: Option<&AugmentDisplay>, x: i32, y: i32) {
        let _ = self.proxy.send_event(OverlayEvent::Update {
            index,
            data: augment.cloned(),
            x,
            y,
        });
    }

    pub fn show(&self, index: usize) {
        let _ = self.proxy.send_event(OverlayEvent::Show(index));
    }

    pub fn hide(&self, index: usize) {
        let _ = self.proxy.send_event(OverlayEvent::Hide(index));
    }

    pub fn hide_all(&self) {
        let _ = self.proxy.send_event(OverlayEvent::HideAll);
    }
}

pub fn calculate_card_positions() -> Option<[(i32, i32); 3]> {
    let game = get_lol_window()?;

    let h = game.height as f32;
    let w = game.width as f32;

    const PANEL_Y_RATIO: f32 = 0.1670;
    const PANEL_W_RATIO: f32 = 0.9528;
    const CARD_GAP_RATIO: f32 = 0.035;

    let panel_w = h * PANEL_W_RATIO;
    let panel_y = game.y as f32 + h * PANEL_Y_RATIO;
    let panel_x = game.x as f32 + (w - panel_w) / 2.0;

    let gap = panel_w * CARD_GAP_RATIO;
    let card_width = (panel_w - gap * 2.0) / 3.0;

    Some([
        ((panel_x + 8.0) as i32, (panel_y + 8.0) as i32),
        ((panel_x + card_width + gap + 8.0) as i32, (panel_y + 8.0) as i32),
        ((panel_x + (card_width + gap) * 2.0 + 8.0) as i32, (panel_y + 8.0) as i32),
    ])
}

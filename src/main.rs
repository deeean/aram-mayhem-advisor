#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

use std::time::{Duration, Instant};
use ocr_rs::{OcrEngine, OcrEngineConfig};
use aram_mayhem_advisor::{capture_augment_cards_fullscreen, overlay::calculate_card_positions_fullscreen, capture_augment_cards, capture::is_lol_foreground, data::{find_augment, Language, MAYHEM_DATA, extract_title_from_ocr, get_champion_name}, live_client::LiveClient, overlay::{Overlay, AugmentDisplay, ChampionStats, calculate_card_positions}, tray::{Tray, TrayCommand}};

const POLL_INTERVAL: Duration = Duration::from_millis(1000);
const CAPTURE_INTERVAL: Duration = Duration::from_millis(50);

const DET_MODEL: &[u8] = include_bytes!("../models/PP-OCRv5_mobile_det.mnn");
const REC_MODEL: &[u8] = include_bytes!("../models/korean_PP-OCRv5_mobile_rec_infer.mnn");
const CHARSET: &[u8] = include_bytes!("../models/ppocr_keys_korean.txt");

fn create_engine() -> OcrEngine {
    let config = OcrEngineConfig::fast();
    OcrEngine::from_bytes(DET_MODEL, REC_MODEL, CHARSET, Some(config))
        .expect("Failed to initialize OCR engine")
}

fn process_cards(
    engine: &OcrEngine,
    cards: &[Option<image::DynamicImage>; 3],
    current_champion: &str,
) -> ([Option<AugmentDisplay>; 3], bool) {
    let mut augments: [Option<AugmentDisplay>; 3] = [None, None, None];
    let mut found_any = false;

    for (i, card_opt) in cards.iter().enumerate() {
        let Some(card) = card_opt else { continue };

        if let Ok(results) = engine.recognize(card) {
            let items: Vec<(&str, i32, i32)> = results
                .iter()
                .map(|r| (r.text.as_str(), r.bbox.rect.top(), r.bbox.rect.left()))
                .collect();

            let title = extract_title_from_ocr(&items, 10);

            if !title.is_empty() {
                if let Some(matched) = find_augment(&title, Language::KoKr, 0.85) {
                    let champ_lower = current_champion.to_lowercase();
                    let champ_display_name = get_champion_name(&champ_lower, Language::KoKr)
                        .unwrap_or_else(|| current_champion.to_string());
                    let champ_info = MAYHEM_DATA.champions.get(&champ_lower)
                        .and_then(|augs| augs.iter().find(|a| a.id == matched.augment.id));

                    let champion_stats = Some(ChampionStats {
                        name: champ_display_name,
                        tier: champ_info.map(|i| i.tier.clone()).unwrap_or_else(|| "-".to_string()),
                        popularity: champ_info.map(|i| i.popularity.clone()).unwrap_or_else(|| "-".to_string()),
                        games: champ_info.map(|i| i.games).unwrap_or(0),
                    });

                    augments[i] = Some(AugmentDisplay {
                        name: matched.augment.name.ko_kr.clone(),
                        tier: matched.augment.tier.clone().unwrap_or_else(|| "-".to_string()),
                        popularity: matched.augment.popularity.clone().unwrap_or_else(|| "-".to_string()),
                        games: matched.augment.games.unwrap_or(0),
                        champion: champion_stats,
                    });
                    found_any = true;
                }
            }
        }
    }

    (augments, found_any)
}

#[cfg(feature = "dev")]
#[tokio::main]
async fn main() {
    let engine = create_engine();
    let overlay = Overlay::new().expect("Failed to create overlay");
    let tray = Tray::new().expect("Failed to create tray");

    let mut last_poll = Instant::now();
    let mut last_capture = Instant::now();

    let champions_list: Vec<String> = MAYHEM_DATA.champions.keys().cloned().collect();
    let mut test_champion_idx = 0usize;
    let mut current_champion = champions_list[test_champion_idx].clone();

    println!("Test mode: champion = {}", current_champion);

    loop {
        if let Some(TrayCommand::Exit) = tray.poll() {
            break;
        }

        let now = Instant::now();

        if now.duration_since(last_poll) >= Duration::from_millis(50) {
            last_poll = now;
            test_champion_idx = (test_champion_idx + 1) % champions_list.len();
            current_champion = champions_list[test_champion_idx].clone();
            println!("Test mode: champion = {}", current_champion);
        }

        if now.duration_since(last_capture) >= CAPTURE_INTERVAL {
            last_capture = now;

            let cards = capture_augment_cards_fullscreen();

            if cards.iter().all(|c| c.is_none()) {
                continue;
            }

            let positions = calculate_card_positions_fullscreen();
            let (augments, found_any) = process_cards(&engine, &cards, &current_champion);

            if found_any {
                for i in 0..3 {
                    if let Some(aug) = &augments[i] {
                        overlay.update(i, Some(aug), positions[i].0, positions[i].1);
                        overlay.show(i);
                    } else {
                        overlay.hide(i);
                    }
                }
            } else {
                overlay.hide_all();
            }
        }

        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(not(feature = "dev"))]
#[tokio::main]
async fn main() {
    let engine = create_engine();
    let client = LiveClient::new().expect("Failed to initialize LiveClient");
    let overlay = Overlay::new().expect("Failed to create overlay");
    let tray = Tray::new().expect("Failed to create tray");

    let mut last_poll = Instant::now();
    let mut last_capture = Instant::now();
    let mut current_champion: Option<String> = None;
    let mut game_active = false;
    let mut overlay_visible = false;
    let mut last_tooltip_state: Option<bool> = None;

    loop {
        if let Some(TrayCommand::Exit) = tray.poll() {
            break;
        }

        let now = Instant::now();

        if now.duration_since(last_poll) >= POLL_INTERVAL {
            last_poll = now;

            match client.poll_game_data().await {
                Some(game_data) => {
                    if !game_active {
                        game_active = true;
                    }

                    let is_mayhem = LiveClient::is_mayhem_mode(&game_data);
                    if last_tooltip_state != Some(is_mayhem) {
                        last_tooltip_state = Some(is_mayhem);
                        if is_mayhem {
                            let champ = current_champion.as_deref().unwrap_or("Unknown");
                            tray.set_tooltip(&format!("무작위 총력전: 아수라장 어드바이저 - 활성화됨 (챔피언: {})", champ));
                        } else {
                            tray.set_tooltip("무작위 총력전: 아수라장 어드바이저");
                        }
                    }

                    if is_mayhem {
                        if let Some(champ) = LiveClient::get_my_champion(&game_data) {
                            if current_champion.as_ref() != Some(&champ) {
                                current_champion = Some(champ);
                            }
                        }
                    } else {
                        overlay.hide_all();
                    }
                }
                None => {
                    if game_active {
                        game_active = false;
                        current_champion = None;
                        last_tooltip_state = None;
                        overlay.hide_all();
                        tray.set_tooltip("무작위 총력전: 아수라장 어드바이저 - 대기 중");
                    }
                }
            }
        }

        if game_active && current_champion.is_some() && now.duration_since(last_capture) >= CAPTURE_INTERVAL {
            last_capture = now;

            if !is_lol_foreground() {
                if overlay_visible {
                    overlay.hide_all();
                    overlay_visible = false;
                }
                continue;
            }

            let cards = capture_augment_cards();

            if cards.iter().all(|c| c.is_none()) {
                continue;
            }

            let Some(positions) = calculate_card_positions() else {
                continue;
            };

            let (augments, found_any) = process_cards(&engine, &cards, current_champion.as_ref().unwrap());

            if found_any {
                overlay_visible = true;
                for i in 0..3 {
                    if let Some(aug) = &augments[i] {
                        overlay.update(i, Some(aug), positions[i].0, positions[i].1);
                        overlay.show(i);
                    } else {
                        overlay.hide(i);
                    }
                }
            } else {
                overlay.hide_all();
            }
        }

        std::thread::sleep(Duration::from_millis(10));
    }
}

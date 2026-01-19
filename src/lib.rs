pub mod data;
pub mod live_client;
pub mod capture;
pub mod overlay;
pub mod tray;

use image::DynamicImage;
use capture::{get_lol_window, capture_region};

const PANEL_Y_RATIO: f32 = 0.1670;
const PANEL_H_RATIO: f32 = 0.4870;
const PANEL_W_RATIO: f32 = 0.9528;
const TEXT_Y_OFFSET_RATIO: f32 = 0.3775;
const TEXT_H_RATIO: f32 = 0.2066;
const CARD_GAP_RATIO: f32 = 0.035;
const CARD_BEZEL_RATIO: f32 = 0.07;

pub fn capture_augment_cards() -> [Option<DynamicImage>; 3] {
    let Some(game) = get_lol_window() else {
        return [None, None, None];
    };

    let h = game.height as f32;
    let w = game.width as f32;

    let panel_h = h * PANEL_H_RATIO;
    let panel_w = h * PANEL_W_RATIO;
    let panel_y = game.y as f32 + h * PANEL_Y_RATIO;
    let panel_x = game.x as f32 + (w - panel_w) / 2.0;

    let gap = panel_w * CARD_GAP_RATIO;
    let card_width = (panel_w - gap * 2.0) / 3.0;
    let bezel = card_width * CARD_BEZEL_RATIO;

    let text_y = panel_y + panel_h * TEXT_Y_OFFSET_RATIO;
    let text_height = panel_h * TEXT_H_RATIO;

    let capture_x = panel_x as i32;
    let capture_y = text_y as i32;
    let capture_w = panel_w as i32;
    let capture_h = text_height as i32;

    let Some(img) = capture_region(capture_x, capture_y, capture_w, capture_h) else {
        return [None, None, None];
    };

    let card_width_full = card_width as u32;
    let bezel = bezel as u32;
    let card_width = card_width as u32 - bezel * 2;
    let gap = gap as u32;
    let text_height = text_height as u32;

    [
        Some(img.crop_imm(bezel, 0, card_width, text_height)),
        Some(img.crop_imm(card_width_full + gap + bezel, 0, card_width, text_height)),
        Some(img.crop_imm((card_width_full + gap) * 2 + bezel, 0, card_width, text_height)),
    ]
}

pub fn crop_augment_cards(img: &DynamicImage) -> [DynamicImage; 3] {
    let (screen_width, screen_height) = (img.width(), img.height());
    let h = screen_height as f32;

    let panel_h = h * PANEL_H_RATIO;
    let panel_w = h * PANEL_W_RATIO;
    let panel_y = h * PANEL_Y_RATIO;
    let panel_x = (screen_width as f32 - panel_w) / 2.0;

    let gap = panel_w * CARD_GAP_RATIO;
    let card_width = (panel_w - gap * 2.0) / 3.0;
    let bezel = card_width * CARD_BEZEL_RATIO;

    let text_y = panel_y + panel_h * TEXT_Y_OFFSET_RATIO;
    let text_height = panel_h * TEXT_H_RATIO;

    let panel_x = panel_x as u32;
    let text_y = text_y as u32;
    let card_width_full = card_width as u32;
    let bezel = bezel as u32;
    let card_width = card_width as u32 - bezel * 2;
    let gap = gap as u32;
    let text_height = text_height as u32;

    [
        img.crop_imm(panel_x + bezel, text_y, card_width, text_height),
        img.crop_imm(panel_x + card_width_full + gap + bezel, text_y, card_width, text_height),
        img.crop_imm(panel_x + (card_width_full + gap) * 2 + bezel, text_y, card_width, text_height),
    ]
}
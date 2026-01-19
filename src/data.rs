use std::sync::LazyLock;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use strsim::jaro_winkler;

pub fn extract_title_from_ocr(items: &[(&str, i32, i32)], y_threshold: i32) -> String {
    if items.is_empty() {
        return String::new();
    }

    let min_top = items.iter().map(|(_, top, _)| *top).min().unwrap_or(0);

    let mut title_parts: Vec<_> = items
        .iter()
        .filter(|(_, top, _)| *top <= min_top + y_threshold)
        .collect();

    title_parts.sort_by_key(|(_, _, left)| *left);

    title_parts
        .iter()
        .map(|(text, _, _)| text.trim())
        .collect::<Vec<_>>()
        .join("")
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    ArAe,
    CsCz,
    DeDe,
    ElGr,
    EnAu,
    EnGb,
    EnPh,
    EnSg,
    EsAr,
    EsEs,
    EsMx,
    FrFr,
    HuHu,
    IdId,
    ItIt,
    JaJp,
    KoKr,
    PlPl,
    PtBr,
    RoRo,
    RuRu,
    ThTh,
    TrTr,
    ViVn,
    ZhCn,
    ZhMy,
    ZhTw,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MayhemData {
    pub champions: HashMap<String, Vec<Champion>>,
    pub augments: Vec<Augment>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Augment {
    pub id: i32,
    pub name: Name,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Name {
    pub ar_ae: String,
    pub cs_cz: String,
    pub de_de: String,
    pub el_gr: String,
    pub en_au: String,
    pub en_gb: String,
    pub en_ph: String,
    pub en_sg: String,
    pub es_ar: String,
    pub es_es: String,
    pub es_mx: String,
    pub fr_fr: String,
    pub hu_hu: String,
    pub id_id: String,
    pub it_it: String,
    pub ja_jp: String,
    pub ko_kr: String,
    pub pl_pl: String,
    pub pt_br: String,
    pub ro_ro: String,
    pub ru_ru: String,
    pub th_th: String,
    pub tr_tr: String,
    pub vi_vn: String,
    pub zh_cn: String,
    pub zh_my: String,
    pub zh_tw: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Champion {
    pub id: i32,
    pub tier: String,
    pub popularity: String,
    pub games: i32,
}

const MAYHEM_JSON: &str = include_str!("../data/mayhem.json");

pub static MAYHEM_DATA: LazyLock<MayhemData> = LazyLock::new(|| {
    serde_json::from_str(MAYHEM_JSON).unwrap()
});

impl Name {
    pub fn get(&self, lang: Language) -> &str {
        match lang {
            Language::ArAe => &self.ar_ae,
            Language::CsCz => &self.cs_cz,
            Language::DeDe => &self.de_de,
            Language::ElGr => &self.el_gr,
            Language::EnAu => &self.en_au,
            Language::EnGb => &self.en_gb,
            Language::EnPh => &self.en_ph,
            Language::EnSg => &self.en_sg,
            Language::EsAr => &self.es_ar,
            Language::EsEs => &self.es_es,
            Language::EsMx => &self.es_mx,
            Language::FrFr => &self.fr_fr,
            Language::HuHu => &self.hu_hu,
            Language::IdId => &self.id_id,
            Language::ItIt => &self.it_it,
            Language::JaJp => &self.ja_jp,
            Language::KoKr => &self.ko_kr,
            Language::PlPl => &self.pl_pl,
            Language::PtBr => &self.pt_br,
            Language::RoRo => &self.ro_ro,
            Language::RuRu => &self.ru_ru,
            Language::ThTh => &self.th_th,
            Language::TrTr => &self.tr_tr,
            Language::ViVn => &self.vi_vn,
            Language::ZhCn => &self.zh_cn,
            Language::ZhMy => &self.zh_my,
            Language::ZhTw => &self.zh_tw,
        }
    }
}

fn normalize_text(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace() && *c != '!' && *c != '?' && *c != '.')
        .collect::<String>()
        .to_lowercase()
}

fn calculate_similarity(ocr_text: &str, augment_name: &str) -> f64 {
    let normalized_ocr = normalize_text(ocr_text);
    let normalized_name = normalize_text(augment_name);

    if normalized_ocr.is_empty() || normalized_name.is_empty() {
        return 0.0;
    }

    let jw_score = jaro_winkler(&normalized_ocr, &normalized_name);

    let len_ocr = normalized_ocr.chars().count() as f64;
    let len_name = normalized_name.chars().count() as f64;
    let len_ratio = len_ocr.min(len_name) / len_ocr.max(len_name);

    jw_score * (0.7 + 0.3 * len_ratio)
}

#[derive(Debug, Clone)]
pub struct AugmentMatch {
    pub augment: Augment,
    pub similarity: f64,
}

pub fn find_augment(ocr_text: &str, lang: Language, threshold: f64) -> Option<AugmentMatch> {
    let data = &*MAYHEM_DATA;
    let normalized_ocr = normalize_text(ocr_text);

    for augment in &data.augments {
        let name = augment.name.get(lang);
        let normalized_name = normalize_text(name);

        if normalized_ocr == normalized_name {
            return Some(AugmentMatch {
                augment: augment.clone(),
                similarity: 1.0,
            });
        }
    }

    let mut best_match: Option<AugmentMatch> = None;

    for augment in &data.augments {
        let name = augment.name.get(lang);
        let similarity = calculate_similarity(ocr_text, name);

        if similarity >= threshold {
            if best_match.as_ref().map_or(true, |m| similarity > m.similarity) {
                best_match = Some(AugmentMatch {
                    augment: augment.clone(),
                    similarity,
                });
            }
        }
    }

    best_match
}

pub fn find_augment_from_candidates(
    ocr_texts: Vec<String>,
    lang: Language,
    threshold: f64
) -> Option<AugmentMatch> {
    ocr_texts.iter()
        .filter_map(|text| find_augment(text, lang, threshold))
        .max_by(|a, b| a.similarity.partial_cmp(&b.similarity).unwrap())
}
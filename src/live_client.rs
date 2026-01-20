use reqwest::Client;
use serde::Deserialize;

const LIVE_CLIENT_URL: &str = "https://127.0.0.1:2999/liveclientdata/allgamedata";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllGameData {
    pub active_player: ActivePlayer,
    pub all_players: Vec<Player>,
    pub game_data: GameData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivePlayer {
    pub summoner_name: String,
    pub riot_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub summoner_name: String,
    pub champion_name: String,
    pub raw_champion_name: String,
    pub team: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameData {
    pub game_mode: String,
    pub game_time: f64,
}

pub struct LiveClient {
    client: Client,
}

impl LiveClient {
    pub fn new() -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        Ok(Self { client })
    }

    pub async fn poll_game_data(&self) -> Option<AllGameData> {
        match self.client.get(LIVE_CLIENT_URL).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    response.json::<AllGameData>().await.ok()
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    pub fn get_my_champion(game_data: &AllGameData) -> Option<String> {
        let my_name = &game_data.active_player.summoner_name;

        game_data
            .all_players
            .iter()
            .find(|p| &p.summoner_name == my_name)
            .map(|p| {
                p.raw_champion_name
                    .strip_prefix("game_character_displayname_")
                    .unwrap_or(&p.raw_champion_name)
                    .to_lowercase()
            })
    }

    pub fn is_mayhem_mode(game_data: &AllGameData) -> bool {
        matches!(game_data.game_data.game_mode.as_str(), "KIWI")
    }
}

impl Default for LiveClient {
    fn default() -> Self {
        Self::new().expect("Failed to create LiveClient")
    }
}

use anyhow::Context;
use serde::de::DeserializeOwned;

pub struct Call {
    api: String,
    path: String,
}

impl Call {
    pub fn new<S: Into<String>>(path: S) -> Self {
        Self {
            api: std::env::var("MINEHUT_URL").unwrap_or_else(|_| "api.dev.minehut.com".to_string()),
            path: path.into(),
        }
    }

    pub async fn get<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
        let full_path = format!("https://{}{}", self.api, self.path);
        log::debug!(target: "MinehutAPI", "Calling API with path: {}", full_path);
        let response = reqwest::get(full_path)
            .await?
            .text().await?;

        serde_json::from_str(&*response).context(format!(
            "Failed to decode data as type T for call {}... {}",
            self.path,
            &*response
        ))
    }
}

macro_rules! get {
    ($function:ident,$response:ty,$url:literal) => {
        pub async fn $function() -> anyhow::Result<$response> {
            Call::new($url)
                .get::<$response>()
                .await
        }
    }
}

#[derive(serde_derive::Deserialize, Debug)]
pub struct NetworkSimpleStatsResponse {
    pub player_count: usize,
    pub server_count: usize,
    pub server_max: usize,
    pub ram_count: usize,
    pub ram_max: usize,
}

get!(get_simple_stats, NetworkSimpleStatsResponse, "/network/simple_stats");

#[derive(serde_derive::Deserialize, Debug)]
pub struct ServerByNameResponse {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "activeServerPlan")]
    pub active_server_plan: Option<String>,
    pub backup_slots: Option<f64>,
    pub categories: Option<Vec<String>>,
    #[serde(rename = "connectedServers")]
    pub connected_servers: Option<Vec<String>>,
    pub creation: Option<f64>,
    pub credits_per_day: Option<f64>,
    pub default_banner_image: Option<String>,
    pub default_banner_tint: Option<String>,
    #[serde(rename = "inheritedCategories")]
    pub inherited_categories: Option<Vec<String>>,
    pub last_online: Option<f64>,
    #[serde(rename = "maxPlayers")]
    pub max_players: Option<f64>,
    pub motd: Option<String>,
    pub name: Option<String>,
    pub name_lower: Option<String>,
    pub online: Option<bool>,
    pub owner: Option<String>,
    pub platform: Option<String>,
    #[serde(rename = "playerCount")]
    pub player_count: Option<f64>,
    pub port: Option<f64>,
    pub proxy: Option<bool>,
    pub purchased_icons: Option<Vec<String>>,
    #[serde(rename = "rawPlan")]
    pub raw_plan: Option<String>,
    pub server_plan: Option<String>,
    pub server_version_type: Option<String>,
    pub storage_node: Option<String>,
    pub suspended: Option<bool>,
    pub visibility: Option<bool>,
}

#[derive(serde_derive::Deserialize, Debug)]
pub struct WrappedServer {
    server: ServerByNameResponse
}

pub async fn get_server_by_name<S: Into<String> + std::fmt::Display>(server: S) -> anyhow::Result<ServerByNameResponse> {
    Call::new(format!("/server/{server}?byName=true"))
        .get::<WrappedServer>()
        .await
        .map(|ws| ws.server)
}

use anyhow::Context;
use serde::de::DeserializeOwned;

struct Call {
    api: String,
    path: String,
}

impl Call {
    pub fn new<S: Into<String>>(path: S) -> Self {
        Self {
            api: std::env::var("MINEHUT_URL").unwrap_or("api.dev.minehut.com".to_string()),
            path: path.into(),
        }
    }

    pub async fn get<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
        let full_path = format!("https://{}{}", self.api, self.path);
        log::debug!(target: "MinehutAPI", "Calling API with path: {}", full_path);
        let response = reqwest::get(full_path).await?.text().await?;

        serde_json::from_str(&*response).context(format!(
            "Failed to decode data as type T for call {}",
            self.path
        ))
    }
}

macro_rules! call {
    (GET $function:ident AS $response:ty = $url:literal) => {
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

call!(GET get_simple_stats AS NetworkSimpleStatsResponse = "/network/simple_stats");

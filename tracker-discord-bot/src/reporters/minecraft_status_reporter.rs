use mc_protocol::ProtocolVersion;
use std::time::Duration;

const TARGET_IP: &'static str = "mh-prd.minehut.com";
const TARGET_PORT: u16 = 25565;
const NATIVE_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::V118R2;

#[derive(serde_derive::Deserialize)]
pub struct StatusBreakdown {
    #[serde(rename = "online")]
    players: usize,
    #[serde(rename = "max")]
    servers: usize,
}

#[derive(serde_derive::Deserialize)]
pub struct StatusResponseFull {
    #[serde(rename = "players")]
    breakdown: StatusBreakdown,
}

#[derive(Debug)]
pub struct MinecraftStats {
    pub players: usize,
    pub servers: usize,
    pub latency: u128,
}

pub async fn query_minecraft_status() -> anyhow::Result<MinecraftStats> {
    let bot_response = crate::minecraft_bot::request_status(TARGET_IP, TARGET_PORT, NATIVE_PROTOCOL_VERSION).await?;
    log::info!("Got response!");
    let full_res = serde_json::from_str::<StatusResponseFull>(
        bot_response.0.json_response.as_ref(),
    )?;
    return Ok(MinecraftStats {
        latency: bot_response.1,
        players: full_res.breakdown.players,
        servers: full_res.breakdown.servers,
    });
}

crate::reporter!(Option<MinecraftStats>, "MinecraftStats", |self| {
    tokio::spawn(async move {
        loop {
            self.emit(match query_minecraft_status().await {
                Ok(x) => Some(x),
                Err(err) => {
                    log::error!("Error calling minecraft status {err:?}");
                    None
                },
            })
            .await;
            // poll every half second
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
    Ok(())
});

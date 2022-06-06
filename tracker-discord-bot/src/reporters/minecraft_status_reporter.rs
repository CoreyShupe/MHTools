use mc_protocol::packets::client_bound::status::{Pong, StatusResponse};
use mc_protocol::packets::server_bound::handshaking::{Handshake, NextState};
use mc_protocol::packets::server_bound::status::{Ping, StatusRequest};
use mc_protocol::{
    BufferState, MinecraftPacketBuffer, PacketToCursor, ProtocolSheet, ProtocolVersion,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

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
    #[derive(Default)]
    struct Context {
        response: Option<StatusResponse>,
        latency: Option<u128>,
        write_queue: Vec<u8>,
    }

    type Sheet = ProtocolSheet<Context>;

    let mut protocol_sheet = ProtocolSheet::new(NATIVE_PROTOCOL_VERSION);

    fn status_response_handle(
        _: &mut Sheet,
        context: &mut Context,
        response: StatusResponse,
    ) -> anyhow::Result<()> {
        context.response = Some(response);
        context.write_queue.append(
            &mut Ping {
                payload: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_millis() as i64,
            }
            .to_cursor(NATIVE_PROTOCOL_VERSION, None, None)?
            .into_inner(),
        );
        Ok(())
    }

    fn pong_handle(_: &mut Sheet, context: &mut Context, pong: Pong) -> anyhow::Result<()> {
        context.latency = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis()
                - pong.payload as u128,
        );
        Ok(())
    }

    protocol_sheet.register_packet_handle(Box::new(status_response_handle));
    protocol_sheet.register_packet_handle(Box::new(pong_handle));

    let handshake = Handshake::new(
        758i32.into(),
        TARGET_IP.into(),
        TARGET_PORT,
        (1i32.into(), NextState::Status {}),
    );
    let status_request = StatusRequest {};

    let mut buffer = MinecraftPacketBuffer::new();

    let mut stream = TcpStream::connect(format!("{TARGET_IP}:{TARGET_PORT}")).await?;
    stream
        .write(
            handshake
                .to_cursor(ProtocolVersion::Handshake, None, None)?
                .into_inner()
                .as_slice(),
        )
        .await?;
    stream
        .write(
            status_request
                .to_cursor(NATIVE_PROTOCOL_VERSION, None, None)?
                .into_inner()
                .as_slice(),
        )
        .await?;

    let mut context = Context {
        response: None,
        latency: None,
        write_queue: Vec::new(),
    };

    loop {
        match buffer.poll() {
            BufferState::PacketReady => {
                protocol_sheet.call_generic(&mut context, buffer.packet_reader()?)?;
                stream.write(&context.write_queue).await?;
                context.write_queue.clear();

                if let (Some(response), Some(latency)) = (&context.response, &context.latency) {
                    let full_res = serde_json::from_str::<StatusResponseFull>(
                        response.json_response.as_ref(),
                    )?;
                    return Ok(MinecraftStats {
                        latency: *latency,
                        players: full_res.breakdown.players,
                        servers: full_res.breakdown.servers,
                    });
                }
            }
            BufferState::Waiting => {
                stream.read_buf(buffer.inner_buf()).await?;
            }
            BufferState::Error(error) => {
                anyhow::bail!("Found error {} while polling buffer.", error);
            }
        }
    }
}

crate::reporter!(Option<MinecraftStats>, "MinecraftStats", |self| {
    tokio::spawn(async move {
        loop {
            self.emit(match query_minecraft_status().await {
                Ok(x) => Some(x),
                Err(_) => None,
            })
            .await;
            // poll every half second
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
    Ok(())
});

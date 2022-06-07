use std::io::Cursor;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use mc_protocol::packets::client_bound::status::{JSONResponse, Pong, StatusResponse};
use mc_protocol::packets::packet_async::ProtocolSheet;
use mc_protocol::{MinecraftPacketBuffer, PacketToCursor, ProtocolVersion, wrap_async_packet_handle};
use mc_protocol::packets::server_bound::handshaking::{Handshake, NextState, ServerAddress};
use mc_protocol::packets::server_bound::status::{Ping, StatusRequest};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

pub fn get_system_time_as_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
}

pub async fn request_status<S: ToString>(
    address: S,
    port: u16,
    protocol_version: ProtocolVersion,
) -> anyhow::Result<(StatusResponse, u128)> {
    fn packet_to_vec<T: PacketToCursor>(protocol: ProtocolVersion, packet: T) -> anyhow::Result<Vec<u8>> {
        packet.to_cursor(protocol, None, None).map(Cursor::into_inner)
    }

    async fn write_packet<T: PacketToCursor + std::fmt::Debug>(protocol: ProtocolVersion, packet: T, write_half: &mut OwnedWriteHalf) -> anyhow::Result<()> {
        let bytes = packet_to_vec(protocol, packet)?;
        use anyhow::Context;
        write_half.write(bytes.as_slice()).await.map(|_| ()).context("Failed to write packet.")
    }

    struct Context {
        response: Option<StatusResponse>,
        pong: Option<Pong>,
        write_half: OwnedWriteHalf,
    }

    wrap_async_packet_handle! {
        fn handle_status_response<Context, StatusResponse>(sheet, context, status) {
            let mut context_write_lock = context.write().await;
            context_write_lock.response = Some(status);
            write_packet(sheet.read().await.protocol_version, Ping {
                payload: get_system_time_as_millis() as i64
            }, &mut context_write_lock.write_half).await?;
        }

        fn handle_pong_response<Context, Pong>(_sheet, context, pong) {
            let mut context_write_lock = context.write().await;
            context_write_lock.pong = Some(pong);
        }
    }

    let mut sheet = ProtocolSheet::<Context>::new(protocol_version);
    sheet.register_packet_handle::<StatusResponse>(handle_status_response);
    sheet.register_packet_handle::<Pong>(handle_pong_response);

    let mut buffer = MinecraftPacketBuffer::new();
    let stream = TcpStream::connect(format!("{}:{}", address.to_string(), port)).await?;

    let (mut read_half, mut write_half) = stream.into_split();

    write_packet(ProtocolVersion::Handshake, Handshake {
        protocol_version: protocol_version.to_spec().0.into(),
        server_address: ServerAddress::from(address.to_string()),
        server_port: port,
        next_state: (1i32.into(), NextState::Status {}),
    }, &mut write_half).await?;
    write_packet(protocol_version, StatusRequest {}, &mut write_half).await?;

    let context = Context {
        response: None,
        pong: None,
        write_half,
    };

    let locked_sheet = Arc::new(RwLock::new(sheet));
    let locked_context = Arc::new(RwLock::new(context));

    while let Ok(pass_back) = buffer.read_to_next_packet(read_half).await {
        read_half = pass_back;
        ProtocolSheet::call_generic(
            Arc::clone(&locked_sheet),
            Arc::clone(&locked_context),
            buffer.packet_reader()?,
        ).await?;

        let read_context = RwLock::read(&locked_context).await;
        if let (Some(_), Some(__)) = (&read_context.response, &read_context.pong) {
            break;
        }
    }

    let context = locked_context.read().await;
    Ok((StatusResponse {
        json_response: JSONResponse::from(context.response.as_ref().map(|res| String::from(&res.json_response)).unwrap()),
    }, get_system_time_as_millis() - (context.pong.as_ref().unwrap().payload as u128)))
}
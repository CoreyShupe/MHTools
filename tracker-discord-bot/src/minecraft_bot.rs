use mc_protocol::ext::write_packet;
use mc_protocol::packets::client_bound::status::{JSONResponse, Pong, StatusResponse};
use mc_protocol::packets::packet_async::ProtocolSheet;
use mc_protocol::packets::server_bound::handshaking::{Handshake, NextState, ServerAddress};
use mc_protocol::packets::server_bound::status::{Ping, StatusRequest};
use mc_protocol::{wrap_async_packet_handle, MinecraftPacketBuffer, ProtocolVersion};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::tcp::OwnedWriteHalf;
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
    struct Context {
        response: Option<StatusResponse>,
        pong: Option<Pong>,
        write_half: OwnedWriteHalf,
    }

    wrap_async_packet_handle! {
        fn handle_status_response<Context, StatusResponse>(sheet, context, status) {
            let mut context_write_lock = context.write().await;
            context_write_lock.response = Some(status);
            write_packet(
                Ping { payload: get_system_time_as_millis() as i64},
                sheet.read().await.protocol_version,
                &mut context_write_lock.write_half
            ).await?;
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

    write_packet(
        Handshake {
            protocol_version: protocol_version.to_spec().0.into(),
            server_address: ServerAddress::from(address.to_string()),
            server_port: port,
            next_state: (1i32.into(), NextState::Status {}),
        },
        ProtocolVersion::Handshake,
        &mut write_half,
    )
        .await?;
    write_packet(StatusRequest {}, protocol_version, &mut write_half).await?;

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
        )
            .await?;

        let read_context = RwLock::read(&locked_context).await;
        if let (Some(_), Some(__)) = (&read_context.response, &read_context.pong) {
            break;
        }
    }

    let context = locked_context.read().await;
    Ok((
        StatusResponse {
            json_response: JSONResponse::from(
                context
                    .response
                    .as_ref()
                    .map(|res| String::from(&res.json_response))
                    .unwrap(),
            ),
        },
        get_system_time_as_millis() - (context.pong.as_ref().unwrap().payload as u128),
    ))
}

use minehut_api::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] ([{}]/[{}]) {}",
                record.level(),
                chrono::Local::now().format("[%H:%M:%S]"),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .apply()?;

    let simple = get_simple_stats()
        .await
        .unwrap();
    log::info!("JSON Response: {simple:?}");

    Ok(())
}

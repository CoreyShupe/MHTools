use minehut_api::prelude::*;
use std::time::Duration;

crate::reporter!(
    Option<NetworkSimpleStatsResponse>,
    "NetworkStats",
    |self| {
        tokio::spawn(async move {
            loop {
                match get_simple_stats().await {
                    Ok(stats) => self.emit(Some(stats)).await,
                    Err(_) => self.emit(None).await,
                };
                // poll every half second
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });
        Ok(())
    }
);

use serenity::futures::future::BoxFuture;
use serenity::prelude::TypeMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::RwLock;

mod network_stats_reporter;

pub use network_stats_reporter::ReporterKey as NetworkStatsReporterKey;

mod minecraft_status_reporter;
pub use minecraft_status_reporter::MinecraftStats;
pub use minecraft_status_reporter::ReporterKey as MinecraftStatsReporterKey;

async fn register_reporter<
    T: Send,
    R: Reporter<T> + Default,
    K: serenity::prelude::TypeMapKey<Value = flume::Receiver<T>>,
>(
    type_map: Arc<RwLock<TypeMap>>,
) {
    let mut reporter = R::default();
    let flume_reporter = reporter.create_reporter();
    let mut write_lock = type_map.write().await;
    write_lock.insert::<K>(flume_reporter);
    if let Err(err) = reporter.boot() {
        log::error!("Failed to boot reporter: {err:?}");
    }
}

pub async fn configure(type_map: Arc<RwLock<TypeMap>>) {
    register_reporter::<
        Option<minehut_api::prelude::NetworkSimpleStatsResponse>,
        network_stats_reporter::Reporter,
        network_stats_reporter::ReporterKey,
    >(Arc::clone(&type_map))
    .await;

    register_reporter::<
        Option<MinecraftStats>,
        minecraft_status_reporter::Reporter,
        minecraft_status_reporter::ReporterKey,
    >(Arc::clone(&type_map))
    .await;
}

pub trait Reporter<T: Send> {
    const NAME: &'static str;

    fn create_reporter(&mut self) -> flume::Receiver<T> {
        let (tx, rx) = flume::unbounded();
        self.populate_sender(tx);
        rx
    }

    fn emit<'emit_life>(&self, item: T) -> BoxFuture<'emit_life, ()>
    where
        T: 'emit_life + Debug,
    {
        if let Some(sender) = self.sender() {
            Box::pin(async move {
                log::info!(target: &*format!("{}/Reporter", Self::NAME), "Emitting item: {item:?}");
                if let Err(err) = sender.send_async(item).await {
                    log::error!(target: &*format!("{}/Reporter", Self::NAME), "Error emitting stats: {:?}", err);
                } else {
                    log::info!(target: &*format!("{}/Reporter", Self::NAME), "Successfully emitted flume message.");
                }
            })
        } else {
            Box::pin(async move {})
        }
    }

    fn populate_sender(&mut self, sender: flume::Sender<T>);

    fn sender(&self) -> Option<flume::Sender<T>>;

    fn boot(self) -> anyhow::Result<()>;
}

#[macro_export]
macro_rules! reporter {
    ($data_type:ty, $name:literal, |$self_ident:ident| {
        $($boot_tokens:tt)+
    }) => {
        pub struct ReporterKey;

        impl serenity::prelude::TypeMapKey for ReporterKey {
            type Value = flume::Receiver<$data_type>;
        }

        #[derive(Default)]
        pub struct Reporter {
            sender: Option<flume::Sender<$data_type>>
        }

        impl crate::reporters::Reporter<$data_type> for Reporter {
            const NAME: &'static str = $name;

            fn populate_sender(&mut self, sender: flume::Sender<$data_type>) {
                self.sender = Some(sender);
            }

            fn boot($self_ident: Self) -> anyhow::Result<()> {
                $($boot_tokens)+
            }

            fn sender(&self) -> Option<flume::Sender<$data_type>> {
                self.sender.as_ref().map(|x| x.clone())
            }
        }
    }
}

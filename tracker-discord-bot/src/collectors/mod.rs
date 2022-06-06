use crate::TypeMap;
use serenity::cache::Cache;
use serenity::http::Http;
use std::sync::Arc;
use tokio::sync::RwLock;

mod builtin_minecraft_stats_monitor;
mod builtin_network_stats_monitor;

pub async fn configure(type_map: Arc<RwLock<TypeMap>>, cache_and_http: (Arc<Cache>, Arc<Http>)) {
    let map_clone = Arc::clone(&type_map);
    let cache_clone = Arc::clone(&cache_and_http.0);
    let http_clone = Arc::clone(&cache_and_http.1);
    tokio::spawn(async move {
        builtin_network_stats_monitor::setup(map_clone, (cache_clone, http_clone)).await
    });
    let map_clone = Arc::clone(&type_map);
    let cache_clone = Arc::clone(&cache_and_http.0);
    let http_clone = Arc::clone(&cache_and_http.1);
    tokio::spawn(async move {
        builtin_minecraft_stats_monitor::setup(map_clone, (cache_clone, http_clone)).await
    });
}

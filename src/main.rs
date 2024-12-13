mod config;
mod error;
mod handlers;
mod models;
mod storage;

use config::Config;
use handlers::{handle_add_document, handle_rejection, handle_search, json_body};
use models::SearchEngine;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::signal::ctrl_c;
use warp::Filter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::load()?;
    tracing::info!("Загружена конфигурация: {:?}", config);

    let search_engine = SearchEngine::new(&config)?;
    let search_engine = Arc::new(search_engine);
    let search_engine_clone = search_engine.clone();

    let search_engine_filter = warp::any().map(move || search_engine.clone());

    let add_document = warp::post()
        .and(warp::path("document"))
        .and(json_body())
        .and(search_engine_filter.clone())
        .and_then(handle_add_document);

    let search = warp::get()
        .and(warp::path("search"))
        .and(warp::query::<HashMap<String, String>>())
        .and(search_engine_filter.clone())
        .and_then(handle_search);

    let routes = add_document.or(search).recover(handle_rejection);

    let addr = (config.server.host, config.server.port);
    tracing::info!(
        "Сервер запущен на http://{}:{}",
        config.server.host,
        config.server.port
    );

    let (_, _) = tokio::join!(
        async move {
            match ctrl_c().await {
                Ok(()) => {
                    tracing::info!("Получен сигнал завершения, закрываем движок...");
                    if let Err(e) = search_engine_clone.close().await {
                        tracing::error!("Ошибка при закрытии движка: {}", e);
                    }
                }
                Err(err) => {
                    tracing::error!("Ошибка при ожидании Ctrl+C: {}", err);
                }
            }
        },
        warp::serve(routes).run(addr)
    );

    Ok(())
}

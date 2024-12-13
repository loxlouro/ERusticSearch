mod storage;
mod handlers;
mod models;
mod error;
mod config;

use handlers::{handle_add_document, handle_search, handle_rejection, json_body};
use models::SearchEngine;
use std::sync::Arc;
use std::collections::HashMap;
use warp::Filter;
use config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Инициализация логирования
    tracing_subscriber::fmt::init();

    // Загрузка конфигурации
    let config = Config::load()?;
    tracing::info!("Загружена конфигурация: {:?}", config);

    // Инициализация поискового движка
    let search_engine = SearchEngine::new(&config)?;
    let search_engine = Arc::new(search_engine);

    // Роуты для API
    let search_engine_filter = warp::any().map(move || search_engine.clone());

    // POST /document - добавить документ
    let add_document = warp::post()
        .and(warp::path("document"))
        .and(json_body())
        .and(search_engine_filter.clone())
        .and_then(handle_add_document);

    // GET /search?q=query - поиск документов
    let search = warp::get()
        .and(warp::path("search"))
        .and(warp::query::<HashMap<String, String>>())
        .and(search_engine_filter.clone())
        .and_then(handle_search);

    let routes = add_document
        .or(search)
        .recover(handle_rejection);

    let addr = (config.server.host, config.server.port);
    tracing::info!("Сервер запущен на http://{}:{}", config.server.host, config.server.port);
    
    warp::serve(routes).run(addr).await;
    Ok(())
}

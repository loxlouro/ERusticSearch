mod storage;
mod handlers;
mod models;
mod error;

use handlers::{handle_add_document, handle_search, handle_rejection};
use models::SearchEngine;
use std::sync::Arc;
use std::collections::HashMap;
use warp::Filter;

#[tokio::main]
async fn main() {
    let search_engine = SearchEngine::new();
    let search_engine = Arc::new(search_engine);

    // Роуты для API
    let search_engine_filter = warp::any().map(move || search_engine.clone());

    // POST /document - добавить документ
    let add_document = warp::post()
        .and(warp::path("document"))
        .and(warp::body::json())
        .and(search_engine_filter.clone())
        .and_then(handle_add_document);

    // GET /search?q=query - поиск документов
    let search = warp::get()
        .and(warp::path("search"))
        .and(warp::query::<HashMap<String, String>>())
        .and(search_engine_filter.clone())
        .and_then(handle_search);

    let routes = add_document.or(search).recover(handle_rejection);

    println!("Сервер запущен на http://localhost:3030");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

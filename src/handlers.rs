use std::collections::HashMap;
use std::sync::Arc;
use warp::{Reply, Rejection};
use crate::models::{SearchEngine, Document};
use crate::error::StorageError;
use serde_json;

pub async fn handle_add_document(
    document: Document,
    search_engine: Arc<SearchEngine>,
) -> Result<impl Reply, Rejection> {
    println!("Получен документ: {:?}", document);
    match search_engine.add_document(document).await {
        Ok(_) => Ok(warp::reply::json(&"Документ успешно добавлен")),
        Err(e) => Err(warp::reject::custom(StorageError(e)))
    }
}

pub async fn handle_search(
    query: HashMap<String, String>,
    search_engine: Arc<SearchEngine>,
) -> Result<impl Reply, Rejection> {
    let q = query.get("q").cloned().unwrap_or_default();
    let results = search_engine.search(&q).await;
    Ok(warp::reply::json(&results))
}

pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    let message = if err.is_not_found() {
        "Путь не найден"
    } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
        println!("Ошибка десериализации: {:?}", e);
        "Неверный формат JSON"
    } else if let Some(e) = err.find::<StorageError>() {
        println!("Ошибка сохранения: {:?}", e);
        "Ошибка сохранения докум��нта"
    } else {
        "Внутренняя ошибка сервера"
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "error": message
        })),
        warp::http::StatusCode::BAD_REQUEST,
    ))
}
 
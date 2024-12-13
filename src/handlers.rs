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
    tracing::info!("Получен документ: {:?}", document);
    
    match search_engine.add_document(document).await {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "status": "success",
                "message": "Документ успешно добавлен"
            })),
            warp::http::StatusCode::CREATED,
        )),
        Err(e) => {
            tracing::error!("Ошибка добавления документа: {}", e);
            Err(warp::reject::custom(StorageError::from(e)))
        }
    }
}

pub async fn handle_search(
    query: HashMap<String, String>,
    search_engine: Arc<SearchEngine>,
) -> Result<impl Reply, Rejection> {
    let q = query.get("q").cloned().unwrap_or_default();
    tracing::info!("Поисковый запрос: {}", q);
    
    match search_engine.search(&q).await {
        Ok(results) => Ok(warp::reply::json(&serde_json::json!({
            "status": "success",
            "results": results,
            "count": results.len()
        }))),
        Err(e) => {
            tracing::error!("Ошибка поиска: {}", e);
            Err(warp::reject::custom(StorageError::from(e)))
        }
    }
}

pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    let (code, message) = if err.is_not_found() {
        (warp::http::StatusCode::NOT_FOUND, "Путь не найден")
    } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
        tracing::error!("Ошибка десериализации: {:?}", e);
        (warp::http::StatusCode::BAD_REQUEST, "Неверный формат JSON")
    } else if let Some(e) = err.find::<StorageError>() {
        tracing::error!("Ошибка хранилища: {:?}", e);
        (warp::http::StatusCode::INTERNAL_SERVER_ERROR, "Ошибка сервера")
    } else {
        tracing::error!("Необработанная ошибка: {:?}", err);
        (warp::http::StatusCode::INTERNAL_SERVER_ERROR, "Внутренняя ошибка сервера")
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "status": "error",
            "message": message
        })),
        code,
    ))
}
 
use std::collections::HashMap;
use std::sync::Arc;
use warp::{Reply, Rejection, Filter, filters::BoxedFilter};
use crate::models::{SearchEngine, Document};
use crate::error::StorageError;
use serde_json;

// Создаем пользовательскую ошибку для десериализации
#[derive(Debug)]
pub struct JsonError {
    message: String,
}

impl warp::reject::Reject for JsonError {}

// Создаем фильтр для обработки JSON с пользовательской обработкой ошибок
pub fn json_body() -> BoxedFilter<(Document,)> {
    warp::body::content_length_limit(1024 * 16)
        .and(warp::body::json())
        .map(|doc: Document| doc)
        .or_else(|rejection: Rejection| async move {
            if let Some(error) = rejection.find::<warp::filters::body::BodyDeserializeError>() {
                let message = error.to_string()
                    .replace("Request body deserialize error: ", "")
                    .replace(" at line 1 column 40", "");
                Err(warp::reject::custom(JsonError { message }))
            } else {
                Err(rejection)
            }
        })
        .boxed()
}

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
    let (code, message): (warp::http::StatusCode, String) = if err.is_not_found() {
        (warp::http::StatusCode::NOT_FOUND, "Путь не найден".to_string())
    } else if let Some(e) = err.find::<JsonError>() {
        tracing::error!("Ошибка JSON: {:?}", e);
        (
            warp::http::StatusCode::BAD_REQUEST,
            e.message.clone()
        )
    } else if let Some(e) = err.find::<StorageError>() {
        tracing::error!("Ошибка хранилища: {:?}", e);
        (
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Ошибка сервера".to_string()
        )
    } else {
        tracing::error!("Необработанная ошибка: {:?}", err);
        (
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Внутренняя ошибка сервера".to_string()
        )
    };

    let error_json = serde_json::json!({
        "status": "error",
        "message": message,
        "code": code.as_u16(),
        "error_type": match code {
            warp::http::StatusCode::BAD_REQUEST => "validation_error",
            warp::http::StatusCode::NOT_FOUND => "not_found",
            _ => "internal_error"
        }
    });

    let mut response = warp::reply::Response::new(serde_json::to_string(&error_json).unwrap().into());
    *response.status_mut() = code;
    response.headers_mut().insert(
        "content-type",
        warp::http::header::HeaderValue::from_static("application/json"),
    );

    Ok(response)
}
 
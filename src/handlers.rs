use std::convert::Infallible;
use warp::{Filter, Rejection, Reply};
use warp::filters::BoxedFilter;
use serde_json::json;
use crate::models::{Document, SearchEngine};
use std::sync::Arc;

#[derive(Debug)]
struct JsonError {
    message: String,
}

impl warp::reject::Reject for JsonError {}

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

pub async fn handle_add_document(doc: Document, engine: Arc<SearchEngine>) -> Result<impl Reply, Rejection> {
    match engine.add_document(doc).await {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&json!({
                "status": "success",
                "message": "Document added successfully"
            })),
            warp::http::StatusCode::CREATED,
        )),
        Err(e) => Ok(warp::reply::with_status(
            warp::reply::json(&json!({
                "status": "error",
                "message": format!("Failed to add document: {}", e)
            })),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub async fn handle_search(params: std::collections::HashMap<String, String>, engine: Arc<SearchEngine>) -> Result<impl Reply, Rejection> {
    let query = params.get("q").cloned().unwrap_or_default();
    
    match engine.search(&query).await {
        Ok(results) => Ok(warp::reply::json(&json!({
            "status": "success",
            "count": results.len(),
            "results": results
        }))),
        Err(e) => Ok(warp::reply::json(&json!({
            "status": "error",
            "message": format!("Search failed: {}", e)
        }))),
    }
}

pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let (code, message, error_type) = if err.is_not_found() {
        (404, "Not Found".to_string(), "not_found")
    } else if let Some(e) = err.find::<JsonError>() {
        (400, e.message.clone(), "validation_error")
    } else if err.find::<warp::reject::PayloadTooLarge>().is_some() {
        (413, "Payload too large".to_string(), "payload_too_large")
    } else {
        (500, "Internal Server Error".to_string(), "internal_error")
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&json!({
            "status": "error",
            "code": code,
            "error_type": error_type,
            "message": message
        })),
        warp::http::StatusCode::from_u16(code).unwrap()
    ))
}
 
use crate::core::{document::Document, search::SearchEngine};
use serde_json::json;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use warp::{Rejection, Reply};

pub async fn handle_add_document(
    doc: Document,
    engine: Arc<SearchEngine>,
) -> Result<impl Reply, Rejection> {
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

pub async fn handle_search(
    params: HashMap<String, String>,
    engine: Arc<SearchEngine>,
) -> Result<impl Reply, Rejection> {
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
    } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
        (400, e.to_string(), "validation_error")
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
        warp::http::StatusCode::from_u16(code).unwrap(),
    ))
}

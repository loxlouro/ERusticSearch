use rust_search::config::Config;
use rust_search::handlers::{handle_add_document, handle_rejection, handle_search, json_body};
use rust_search::models::{Document, SearchEngine};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::tempdir;
use warp::test::request;
use warp::Filter;

pub fn create_test_config() -> Config {
    let temp_dir = tempdir().unwrap();
    let data_path = temp_dir.path().join("test_data.db");
    let index_path = temp_dir.path().join("test_index");

    Config {
        server: rust_search::config::ServerConfig {
            host: "127.0.0.1".parse().unwrap(),
            port: 3030,
        },
        storage: rust_search::config::StorageConfig {
            data_file: data_path.to_str().unwrap().to_string(),
            index_path: index_path.to_str().unwrap().to_string(),
        },
    }
}

pub fn create_test_document(id: &str, content: &str) -> Document {
    let mut metadata = HashMap::new();
    metadata.insert("author".to_string(), "Test Author".to_string());
    metadata.insert("type".to_string(), "test".to_string());

    Document {
        id: id.to_string(),
        content: content.to_string(),
        metadata,
    }
}

pub async fn create_test_filter() -> impl warp::Filter<Extract = impl warp::Reply> + Clone {
    let config = create_test_config();
    let search_engine = SearchEngine::new(&config).unwrap();
    let search_engine = Arc::new(search_engine);

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

    add_document.or(search).recover(handle_rejection)
}

#[tokio::test]
async fn test_add_document_api() {
    let api = create_test_filter().await;

    let doc = json!({
        "id": "test1",
        "content": "This is a test document for API testing",
        "metadata": {
            "author": "API Tester",
            "type": "test"
        }
    });

    let response = request()
        .method("POST")
        .path("/document")
        .json(&doc)
        .reply(&api)
        .await;

    println!("\n=== Add Document Response ===");
    println!("Status: {}", response.status());
    println!("Headers: {:#?}", response.headers());
    println!("Body: {}", String::from_utf8_lossy(response.body()));
    println!("========================\n");

    assert_eq!(response.status(), 201);

    let response_data: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
    assert_eq!(response_data["status"], "success");
}

#[tokio::test]
async fn test_search_api() {
    let api = create_test_filter().await;

    let doc = json!({
        "id": "test2",
        "content": "Unique test content for search API testing",
        "metadata": {
            "author": "Search Tester"
        }
    });

    let add_response = request()
        .method("POST")
        .path("/document")
        .json(&doc)
        .reply(&api)
        .await;

    assert_eq!(add_response.status(), 201);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let response = request()
        .method("GET")
        .path("/search?q=unique")
        .reply(&api)
        .await;

    println!("\n=== Search Response ===");
    println!("Status: {}", response.status());
    println!("Headers: {:#?}", response.headers());
    println!("Body: {}", String::from_utf8_lossy(response.body()));
    println!("========================\n");

    assert_eq!(response.status(), 200);

    let response_data: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
    assert_eq!(response_data["status"], "success");
    assert!(!response_data["results"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_invalid_document() {
    let api = create_test_filter().await;

    let response = request()
        .method("POST")
        .path("/document")
        .header("Content-Type", "application/json")
        .json(&json!({
            "content": "Test content",
            "metadata": {}
        }))
        .reply(&api)
        .await;

    println!("\n=== Invalid Document Response ===");
    println!("Status: {}", response.status());
    println!("Headers: {:#?}", response.headers());
    println!("Body: {}", String::from_utf8_lossy(response.body()));
    println!("========================\n");

    assert_eq!(response.status(), 400);

    let response_data: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
    assert_eq!(response_data["status"], "error");
    assert_eq!(response_data["code"], 400);
    assert_eq!(response_data["error_type"], "validation_error");
}

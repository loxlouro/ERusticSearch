#[cfg(test)]
mod tests {
    use crate::models::{Document, SearchEngine};
    use crate::config::Config;
    use crate::handlers::{handle_add_document, handle_search, handle_rejection};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tempfile::tempdir;
    use warp::test::request;
    use warp::Filter;
    use serde_json::json;

    fn create_test_config() -> Config {
        let temp_dir = tempdir().unwrap();
        let data_path = temp_dir.path().join("test_data.db");
        let index_path = temp_dir.path().join("test_index");

        Config {
            server: crate::config::ServerConfig {
                host: "127.0.0.1".parse().unwrap(),
                port: 3030,
            },
            storage: crate::config::StorageConfig {
                data_file: data_path.to_str().unwrap().to_string(),
                index_path: index_path.to_str().unwrap().to_string(),
            },
        }
    }

    mod api {
        use super::*;

        async fn create_test_filter() -> impl warp::Filter<Extract = impl warp::Reply> + Clone {
            let config = create_test_config();
            let search_engine = SearchEngine::new(&config).unwrap();
            let search_engine = Arc::new(search_engine);
            
            let search_engine_filter = warp::any().map(move || search_engine.clone());

            // POST /document
            let add_document = warp::post()
                .and(warp::path("document"))
                .and(crate::handlers::json_body())
                .and(search_engine_filter.clone())
                .and_then(handle_add_document);

            // GET /search?q=query
            let search = warp::get()
                .and(warp::path("search"))
                .and(warp::query::<HashMap<String, String>>())
                .and(search_engine_filter.clone())
                .and_then(handle_search);

            // Объединяем маршруты и добавляем обработку ошибок
            add_document
                .or(search)
                .recover(handle_rejection)
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

            // Сначала добавим документ
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
            
            println!("\n=== Add Document Response (Search Test) ===");
            println!("Status: {}", add_response.status());
            println!("Headers: {:#?}", add_response.headers());
            println!("Body: {}", String::from_utf8_lossy(add_response.body()));
            println!("========================\n");

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
            
            let results = response_data["results"].as_array().unwrap();
            assert!(!results.is_empty(), "Результаты поиска не должны быть пустыми");
            assert_eq!(results[0]["id"], "test2");
            assert!(results[0]["content"].as_str().unwrap().contains("Unique"));
        }

        #[tokio::test]
        async fn test_search_no_results() {
            let api = create_test_filter().await;

            let response = request()
                .method("GET")
                .path("/search?q=nonexistent")
                .reply(&api)
                .await;

            println!("\n=== Empty Search Response ===");
            println!("Status: {}", response.status());
            println!("Headers: {:#?}", response.headers());
            println!("Body: {}", String::from_utf8_lossy(response.body()));
            println!("========================\n");

            assert_eq!(response.status(), 200);
            
            let response_data: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
            assert_eq!(response_data["status"], "success");
            assert_eq!(response_data["count"], 0);
            assert!(response_data["results"].as_array().unwrap().is_empty());
        }

        #[tokio::test]
        async fn test_invalid_document() {
            let api = create_test_filter().await;

            // Создаем некорректный JSON с отсутствующим обязательным полем id
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

            println!("\n=== Response Debug Info ===");
            println!("Status: {}", response.status());
            println!("Headers: {:#?}", response.headers());
            println!("Body: {}", String::from_utf8_lossy(response.body()));
            println!("========================\n");

            assert_eq!(response.status(), 400);
            
            // Проверяем, что ответ содержит правильный Content-Type
            assert_eq!(
                response.headers().get("content-type").unwrap(),
                "application/json"
            );
            
            // Преобразуем тело ответа в строку для отладки
            let body = String::from_utf8_lossy(response.body());
            println!("Response body: {}", body);

            // Пробуем распарсить JSON с обработкой ошибок
            let response_data: serde_json::Value = match serde_json::from_slice(response.body()) {
                Ok(data) => data,
                Err(e) => {
                    println!("Failed to parse JSON: {}", e);
                    println!("Raw response: {:?}", body);
                    panic!("JSON parsing failed");
                }
            };

            // Проверяем структуру ответа
            assert_eq!(response_data["status"], "error");
            assert_eq!(response_data["code"], 400);
            assert_eq!(response_data["error_type"], "validation_error");
            
            // Проверяем сообщение об ошибке
            let error_message = response_data["message"].as_str().expect("message should be a string");
            assert!(error_message.contains("missing field"),
                   "Unexpected error message: {}", error_message);
        }

        #[tokio::test]
        async fn test_multiple_documents_search() {
            let api = create_test_filter().await;

            // Добавляем несколько документов
            for i in 1..=3 {
                let doc = json!({
                    "id": format!("multi{}", i),
                    "content": format!("Common text with unique part {}", i),
                    "metadata": {
                        "author": format!("Author {}", i)
                    }
                });

                let add_response = request()
                    .method("POST")
                    .path("/document")
                    .json(&doc)
                    .reply(&api)
                    .await;

                println!("\n=== Add Document {} Response ===", i);
                println!("Status: {}", add_response.status());
                println!("Headers: {:#?}", add_response.headers());
                println!("Body: {}", String::from_utf8_lossy(add_response.body()));
                println!("========================\n");
            }

            let response = request()
                .method("GET")
                .path("/search?q=Common")
                .reply(&api)
                .await;

            println!("\n=== Multiple Documents Search Response ===");
            println!("Status: {}", response.status());
            println!("Headers: {:#?}", response.headers());
            println!("Body: {}", String::from_utf8_lossy(response.body()));
            println!("========================\n");

            assert_eq!(response.status(), 200);
            
            let response_data: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
            assert_eq!(response_data["count"], 3);
        }
    }

    #[tokio::test]
    async fn test_add_and_search_document() {
        let config = create_test_config();
        let engine = SearchEngine::new(&config).unwrap();

        // Создаем тестовый документ
        let mut metadata = HashMap::new();
        metadata.insert("author".to_string(), "Test Author".to_string());
        
        let doc = Document {
            id: "1".to_string(),
            content: "Rust is a great programming language".to_string(),
            metadata,
        };

        // Добавляем документ
        engine.add_document(doc.clone()).await.unwrap();

        // Ищем документ
        let results = engine.search("Rust").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "1");
        assert!(results[0].content.contains("Rust"));
    }

    #[tokio::test]
    async fn test_search_nonexistent() {
        let config = create_test_config();
        let engine = SearchEngine::new(&config).unwrap();

        let results = engine.search("nonexistent").await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_multiple_documents() {
        let config = create_test_config();
        let engine = SearchEngine::new(&config).unwrap();

        // Добавляем несколько документов
        for i in 1..=3 {
            let mut metadata = HashMap::new();
            metadata.insert("author".to_string(), format!("Author {}", i));
            
            let doc = Document {
                id: i.to_string(),
                content: format!("Document {} content with some text", i),
                metadata,
            };
            engine.add_document(doc).await.unwrap();
        }

        // Проверяем поиск
        let results = engine.search("content").await.unwrap();
        assert_eq!(results.len(), 3);
    }
} 
use rust_search::common::config::Config;
use rust_search::{Document, SearchEngine};
use std::collections::HashMap;
use tempfile::tempdir;

fn create_test_config() -> Config {
    let temp_dir = tempdir().unwrap();
    let data_path = temp_dir.path().join("test_data.db");
    let index_path = temp_dir.path().join("test_index");

    Config {
        server: rust_search::common::config::ServerConfig {
            host: "127.0.0.1".parse().unwrap(),
            port: 3030,
        },
        storage: rust_search::common::config::StorageConfig {
            data_file: data_path.to_str().unwrap().to_string(),
            index_path: index_path.to_str().unwrap().to_string(),
        },
    }
}

fn create_test_document(id: &str, content: &str) -> Document {
    let mut metadata = HashMap::new();
    metadata.insert("author".to_string(), "Test Author".to_string());
    metadata.insert("type".to_string(), "test".to_string());

    Document {
        id: id.to_string(),
        content: content.to_string(),
        metadata,
    }
}

#[tokio::test]
async fn test_add_and_search_document() {
    let config = create_test_config();
    let engine = SearchEngine::new(&config).unwrap();

    let doc = create_test_document("1", "Rust is a great programming language");
    engine.add_document(doc.clone()).await.unwrap();

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

    for i in 1..=3 {
        let doc = create_test_document(
            &format!("doc{}", i),
            &format!("Document {} with some common text", i),
        );
        engine.add_document(doc).await.unwrap();
    }

    let results = engine.search("common").await.unwrap();
    assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn test_metadata_search() {
    let config = create_test_config();
    let engine = SearchEngine::new(&config).unwrap();

    let mut doc = create_test_document("meta1", "Test document with metadata");
    doc.metadata
        .insert("category".to_string(), "test_category".to_string());

    engine.add_document(doc).await.unwrap();

    let results = engine.search("category:test_category").await.unwrap();
    assert!(!results.is_empty());
    assert_eq!(
        results[0].metadata.get("category").unwrap(),
        "test_category"
    );

    let results = engine
        .search("metadata category:test_category")
        .await
        .unwrap();
    assert!(!results.is_empty());

    let results = engine.search("nonexistent_category:value").await.unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_metadata_search_with_field_syntax() -> anyhow::Result<()> {
    let config = create_test_config();
    let engine = SearchEngine::new(&config)?;

    let mut doc1 = Document {
        id: "test1".to_string(),
        content: "test content".to_string(),
        metadata: HashMap::new(),
    };
    doc1.metadata
        .insert("author".to_string(), "John Doe".to_string());

    let mut doc2 = Document {
        id: "test2".to_string(),
        content: "other content".to_string(),
        metadata: HashMap::new(),
    };
    doc2.metadata
        .insert("author".to_string(), "Jane Smith".to_string());

    engine.add_document(doc1.clone()).await?;
    engine.add_document(doc2.clone()).await?;

    let results = engine.search_with_metadata("John", &["author"]).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "test1");

    let results = engine.search_with_metadata("Jane", &["author"]).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "test2");

    Ok(())
}

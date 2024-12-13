#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Document, SearchEngine};
    use crate::config::Config;
    use std::collections::HashMap;
    use tempfile::tempdir;

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
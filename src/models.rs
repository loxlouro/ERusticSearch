use crate::config::Config;
use crate::storage;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::ops::Deref;
use std::sync::Arc;
use tantivy::{
    schema::{Schema, STORED, TEXT},
    Document as TantivyDoc, Index, IndexWriter,
};
use tokio::sync::RwLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

pub struct SearchIndex {
    index: Index,
    writer: Arc<RwLock<IndexWriter>>,
    schema: Schema,
}

impl SearchIndex {
    pub fn new(config: &Config) -> Result<Self> {
        let mut schema_builder = Schema::builder();
        let _id_field = schema_builder.add_text_field("id", TEXT | STORED);
        let _content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let schema = schema_builder.build();

        fs::create_dir_all(&config.storage.index_path)?;

        let index = match Index::open_in_dir(&config.storage.index_path) {
            Ok(index) => index,
            Err(_) => Index::create_in_dir(&config.storage.index_path, schema.clone())?,
        };

        let writer = index.writer(50_000_000)?;

        Ok(SearchIndex {
            index,
            writer: Arc::new(RwLock::new(writer)),
            schema,
        })
    }

    pub async fn add_document(&self, doc: &Document) -> Result<()> {
        let mut tantivy_doc = TantivyDoc::new();
        let id_field = self.schema.get_field("id").unwrap();
        let content_field = self.schema.get_field("content").unwrap();

        tantivy_doc.add_text(id_field, &doc.id);
        tantivy_doc.add_text(content_field, &doc.content);

        let mut writer = self.writer.write().await;
        writer.add_document(tantivy_doc)?;
        writer.commit()?;

        Ok(())
    }

    pub fn search(&self, query: &str) -> Result<Vec<String>> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();
        let content_field = self.schema.get_field("content").unwrap();

        let query_parser = tantivy::query::QueryParser::for_index(&self.index, vec![content_field]);
        let query = query_parser.parse_query(query)?;

        let top_docs = searcher.search(&query, &tantivy::collector::TopDocs::with_limit(10))?;

        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)?;
            let id_field = self.schema.get_field("id").unwrap();
            if let Some(id) = retrieved_doc.get_first(id_field) {
                results.push(id.as_text().unwrap().to_string());
            }
        }

        Ok(results)
    }

    #[allow(dead_code)]
    pub async fn close(&self) -> Result<()> {
        let mut writer = self.writer.write().await;
        writer.commit()?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct SearchEngine {
    documents: Arc<RwLock<HashMap<String, Document>>>,
    search_index: Arc<SearchIndex>,
    config: Config,
}

impl SearchEngine {
    pub fn new(config: &Config) -> Result<Self> {
        let documents = match storage::load_documents(&config.storage.data_file) {
            Ok(docs) => docs,
            Err(e) => {
                tracing::error!("Ошибка загрузки документов: {}", e);
                HashMap::new()
            }
        };

        let search_index = SearchIndex::new(config)?;

        Ok(SearchEngine {
            documents: Arc::new(RwLock::new(documents)),
            search_index: Arc::new(search_index),
            config: config.clone(),
        })
    }

    pub async fn add_document(&self, doc: Document) -> Result<()> {
        self.search_index.add_document(&doc).await?;

        let mut docs = self.documents.write().await;
        docs.insert(doc.id.clone(), doc);
        storage::save_documents(docs.deref(), &self.config.storage.data_file).await?;

        Ok(())
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Document>> {
        let ids = self.search_index.search(query)?;
        let docs = self.documents.read().await;

        Ok(ids
            .into_iter()
            .filter_map(|id| docs.get(&id).cloned())
            .collect())
    }

    #[allow(dead_code)]
    pub async fn search_with_metadata(
        &self,
        _query: &str,
        _fields: &[&str],
    ) -> Result<Vec<Document>> {
        unimplemented!("Search with metadata not implemented yet");
    }

    #[allow(dead_code)]
    async fn add_metadata_field(&self, _field_name: &str) -> Result<()> {
        unimplemented!("Adding metadata fields not implemented yet");
    }

    #[allow(dead_code)]
    async fn update_index_schema(&self) -> Result<()> {
        unimplemented!("Schema updates not implemented yet");
    }

    #[allow(dead_code)]
    pub async fn close(&self) -> Result<()> {
        self.search_index.close().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_config() -> Config {
        let temp_dir = tempdir().unwrap();
        Config {
            server: crate::config::ServerConfig {
                host: "127.0.0.1".parse().unwrap(),
                port: 3030,
            },
            storage: crate::config::StorageConfig {
                data_file: temp_dir
                    .path()
                    .join("test_data.db")
                    .to_str()
                    .unwrap()
                    .to_string(),
                index_path: temp_dir
                    .path()
                    .join("test_index")
                    .to_str()
                    .unwrap()
                    .to_string(),
            },
        }
    }

    #[tokio::test]
    async fn test_engine_creation() {
        let config = create_test_config();
        let engine = SearchEngine::new(&config).unwrap();
        assert!(engine.documents.read().await.is_empty());
        engine.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_document_persistence() {
        let config = create_test_config();
        {
            let engine = SearchEngine::new(&config).unwrap();

            let doc = Document {
                id: "test1".to_string(),
                content: "test content".to_string(),
                metadata: HashMap::new(),
            };

            engine.add_document(doc.clone()).await.unwrap();
            engine.close().await.unwrap();
        }

        let engine2 = SearchEngine::new(&config).unwrap();
        let docs = engine2.documents.read().await;
        assert!(docs.contains_key("test1"));
        let doc = docs.get("test1").unwrap();
        assert_eq!(doc.content, "test content");

        engine2.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_engine_close_and_reopen() {
        let config = create_test_config();

        {
            let engine = SearchEngine::new(&config).unwrap();
            engine.close().await.unwrap();
        }

        let engine2 = SearchEngine::new(&config).unwrap();
        engine2.close().await.unwrap();
    }
}

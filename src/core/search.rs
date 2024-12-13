use super::document::Document;
use super::index::SearchIndex;
use crate::common::config::Config;
use crate::storage::persistence;
use anyhow::Result;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct SearchEngine {
    documents: Arc<RwLock<HashMap<String, Document>>>,
    search_index: Arc<SearchIndex>,
    config: Config,
}

impl SearchEngine {
    pub fn new(config: &Config) -> Result<Self> {
        let documents = match persistence::load_documents(&config.storage.data_file) {
            Ok(docs) => docs,
            Err(e) => {
                tracing::error!("Ошибка загрузки документов: {}", e);
                HashMap::new()
            }
        };

        let search_index = SearchIndex::new(&config.storage.index_path)?;

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
        persistence::save_documents(docs.deref(), &self.config.storage.data_file).await?;

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

    pub async fn close(&self) -> Result<()> {
        self.search_index.close().await?;
        Ok(())
    }

    pub async fn search_with_metadata(
        &self,
        query: &str,
        fields: &[&str],
    ) -> Result<Vec<Document>> {
        let ids = self.search_index.search_with_metadata(query, fields)?;
        let docs = self.documents.read().await;

        Ok(ids
            .into_iter()
            .filter_map(|id| docs.get(&id).cloned())
            .collect())
    }

    pub async fn add_metadata_field(&self, field_name: &str) -> Result<()> {
        self.search_index.add_metadata_field(field_name).await?;
        Ok(())
    }

    pub async fn update_index_schema(&self) -> Result<()> {
        self.search_index.update_schema().await?;
        Ok(())
    }
}

use std::collections::HashMap;
use std::sync::Arc;
use std::ops::Deref;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use crate::storage;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Clone)]
pub struct SearchEngine {
    documents: Arc<RwLock<HashMap<String, Document>>>,
}

impl SearchEngine {
    pub fn new() -> Self {
        let documents = match storage::load_documents() {
            Ok(docs) => docs,
            Err(e) => {
                eprintln!("Ошибка загрузки документов: {}", e);
                HashMap::new()
            }
        };
        
        SearchEngine {
            documents: Arc::new(RwLock::new(documents)),
        }
    }

    pub async fn add_document(&self, doc: Document) -> std::io::Result<()> {
        let mut docs = self.documents.write().await;
        docs.insert(doc.id.clone(), doc);
        storage::save_documents(docs.deref()).await
    }

    pub async fn search(&self, query: &str) -> Vec<Document> {
        let docs = self.documents.read().await;
        docs.values()
            .filter(|doc| doc.content.to_lowercase().contains(&query.to_lowercase()))
            .cloned()
            .collect()
    }
} 
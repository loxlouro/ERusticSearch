use std::collections::HashMap;
use std::sync::Arc;
use std::ops::Deref;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use tantivy::{
    schema::{Schema, TEXT, STORED},
    Index, IndexWriter, Document as TantivyDoc,
};
use crate::storage;
use crate::config::Config;
use anyhow::Result;
use std::fs;

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

        let index = Index::create_in_dir(&config.storage.index_path, schema.clone())?;
        let writer = index.writer(50_000_000)?; // 50MB buffer

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
        
        Ok(ids.into_iter()
            .filter_map(|id| docs.get(&id).cloned())
            .collect())
    }
} 
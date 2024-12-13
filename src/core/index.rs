use super::document::Document;
use anyhow::Result;
use std::fs;
use std::sync::Arc;
use tantivy::{
    schema::{Schema, STORED, TEXT},
    Document as TantivyDoc, Index, IndexWriter,
};
use tokio::sync::RwLock;

pub struct SearchIndex {
    index: Index,
    writer: Arc<RwLock<IndexWriter>>,
    schema: Schema,
}

impl SearchIndex {
    pub fn new(index_path: &str) -> Result<Self> {
        let mut schema_builder = Schema::builder();
        let _id_field = schema_builder.add_text_field("id", TEXT | STORED);
        let _content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let _author_field = schema_builder.add_text_field("author", TEXT | STORED);
        let _type_field = schema_builder.add_text_field("type", TEXT | STORED);
        let schema = schema_builder.build();

        fs::create_dir_all(index_path)?;

        let index = match Index::open_in_dir(index_path) {
            Ok(index) => index,
            Err(_) => Index::create_in_dir(index_path, schema.clone())?,
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

        for (key, value) in &doc.metadata {
            if let Some(field) = self.schema.get_field(key) {
                tantivy_doc.add_text(field, value);
            }
        }

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

    pub async fn close(&self) -> Result<()> {
        let mut writer = self.writer.write().await;
        writer.commit()?;
        Ok(())
    }

    pub fn search_with_metadata(&self, query: &str, fields: &[&str]) -> Result<Vec<String>> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();
        let content_field = self.schema.get_field("content").unwrap();

        let mut search_fields = vec![content_field];
        for field in fields {
            if let Some(field) = self.schema.get_field(field) {
                search_fields.push(field);
            }
        }

        let query_parser = tantivy::query::QueryParser::for_index(&self.index, search_fields);
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

    pub async fn add_metadata_field(&self, field_name: &str) -> Result<()> {
        let mut writer = self.writer.write().await;
        let mut schema_builder = Schema::builder();

        for (_field, field_entry) in self.schema.fields() {
            schema_builder.add_field(field_entry.clone());
        }

        schema_builder.add_text_field(field_name, TEXT | STORED);
        schema_builder.build();

        writer.commit()?;
        Ok(())
    }

    pub async fn update_schema(&self) -> Result<()> {
        let mut writer = self.writer.write().await;
        writer.commit()?;
        Ok(())
    }
}

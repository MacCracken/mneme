//! Full-text search engine backed by Tantivy.

use std::path::Path;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, STORED, STRING, Schema, TEXT, Value};
use tantivy::{Directory, Index, IndexReader, IndexWriter, TantivyDocument};
use uuid::Uuid;

use crate::SearchError;

/// A search result with relevance score.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub note_id: Uuid,
    pub title: String,
    pub path: String,
    pub snippet: String,
    pub score: f32,
}

/// Full-text search engine wrapping a Tantivy index.
pub struct SearchEngine {
    index: Index,
    reader: IndexReader,
    #[allow(dead_code)]
    schema: Schema,
    f_id: Field,
    f_title: Field,
    f_body: Field,
    f_tags: Field,
    f_path: Field,
}

impl SearchEngine {
    /// Open or create a search index at the given directory.
    pub fn open(index_dir: &Path) -> Result<Self, SearchError> {
        let (schema, f_id, f_title, f_body, f_tags, f_path) = build_schema();

        std::fs::create_dir_all(index_dir).map_err(|e| SearchError::Index(e.to_string()))?;

        let dir = tantivy::directory::MmapDirectory::open(index_dir)
            .map_err(|e| SearchError::Index(e.to_string()))?;

        let index = if dir
            .exists(&std::path::PathBuf::from("meta.json"))
            .unwrap_or(false)
        {
            Index::open(dir).map_err(|e| SearchError::Index(e.to_string()))?
        } else {
            Index::create_in_dir(index_dir, schema.clone())?
        };

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            schema,
            f_id,
            f_title,
            f_body,
            f_tags,
            f_path,
        })
    }

    /// Open an in-memory index (for testing).
    pub fn in_memory() -> Result<Self, SearchError> {
        let (schema, f_id, f_title, f_body, f_tags, f_path) = build_schema();
        let index = Index::create_in_ram(schema.clone());
        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::Manual)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            schema,
            f_id,
            f_title,
            f_body,
            f_tags,
            f_path,
        })
    }

    /// Index (or re-index) a note.
    pub fn index_note(
        &self,
        id: Uuid,
        title: &str,
        body: &str,
        tags: &[String],
        path: &str,
    ) -> Result<(), SearchError> {
        let mut writer = self.writer()?;

        // Remove old version if it exists
        let id_term = tantivy::Term::from_field_text(self.f_id, &id.to_string());
        writer.delete_term(id_term);

        let mut doc = TantivyDocument::new();
        doc.add_text(self.f_id, id.to_string());
        doc.add_text(self.f_title, title);
        doc.add_text(self.f_body, body);
        doc.add_text(self.f_tags, tags.join(" "));
        doc.add_text(self.f_path, path);

        writer.add_document(doc)?;
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Remove a note from the index.
    pub fn remove_note(&self, id: Uuid) -> Result<(), SearchError> {
        let mut writer = self.writer()?;
        let id_term = tantivy::Term::from_field_text(self.f_id, &id.to_string());
        writer.delete_term(id_term);
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Search notes by query string.
    ///
    /// Searches across title, body, and tags fields.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let searcher = self.reader.searcher();

        let parser =
            QueryParser::for_index(&self.index, vec![self.f_title, self.f_body, self.f_tags]);

        let parsed = parser
            .parse_query(query)
            .map_err(|e| SearchError::QueryParse(e.to_string()))?;

        let top_docs = searcher.search(&parsed, &TopDocs::with_limit(limit))?;

        let mut results = Vec::with_capacity(top_docs.len());
        for (score, addr) in top_docs {
            let doc: TantivyDocument = searcher.doc(addr)?;

            let id_str = doc
                .get_first(self.f_id)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let title = doc
                .get_first(self.f_title)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let path = doc
                .get_first(self.f_path)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let body = doc
                .get_first(self.f_body)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Simple snippet: first 200 chars of body
            let snippet = if body.len() > 200 {
                format!("{}...", &body[..200])
            } else {
                body.to_string()
            };

            if let Ok(note_id) = Uuid::parse_str(id_str) {
                results.push(SearchResult {
                    note_id,
                    title,
                    path,
                    snippet,
                    score,
                });
            }
        }

        Ok(results)
    }

    fn writer(&self) -> Result<IndexWriter, SearchError> {
        // 50MB heap for the writer
        self.index
            .writer(50_000_000)
            .map_err(|e| SearchError::Index(e.to_string()))
    }
}

fn build_schema() -> (Schema, Field, Field, Field, Field, Field) {
    let mut builder = Schema::builder();
    let f_id = builder.add_text_field("id", STRING | STORED);
    let f_title = builder.add_text_field("title", TEXT | STORED);
    let f_body = builder.add_text_field("body", TEXT | STORED);
    let f_tags = builder.add_text_field("tags", TEXT | STORED);
    let f_path = builder.add_text_field("path", TEXT | STORED);
    (builder.build(), f_id, f_title, f_body, f_tags, f_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_engine() -> SearchEngine {
        SearchEngine::in_memory().unwrap()
    }

    #[test]
    fn index_and_search() {
        let engine = test_engine();
        let id = Uuid::new_v4();

        engine
            .index_note(
                id,
                "Rust Programming",
                "Rust is a systems language",
                &["rust".into(), "programming".into()],
                "rust.md",
            )
            .unwrap();

        let results = engine.search("rust", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].note_id, id);
        assert_eq!(results[0].title, "Rust Programming");
    }

    #[test]
    fn search_by_tag() {
        let engine = test_engine();
        let id = Uuid::new_v4();

        engine
            .index_note(id, "My Note", "Some content", &["agnos".into()], "note.md")
            .unwrap();

        let results = engine.search("agnos", 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_no_results() {
        let engine = test_engine();
        let results = engine.search("nonexistent", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn remove_note_from_index() {
        let engine = test_engine();
        let id = Uuid::new_v4();

        engine
            .index_note(id, "Temporary", "Will be removed", &[], "temp.md")
            .unwrap();
        assert_eq!(engine.search("temporary", 10).unwrap().len(), 1);

        engine.remove_note(id).unwrap();
        assert_eq!(engine.search("temporary", 10).unwrap().len(), 0);
    }

    #[test]
    fn reindex_updates_content() {
        let engine = test_engine();
        let id = Uuid::new_v4();

        engine
            .index_note(id, "Original", "old content about cats", &[], "note.md")
            .unwrap();
        assert_eq!(engine.search("cats", 10).unwrap().len(), 1);

        // Re-index with new content
        engine
            .index_note(id, "Updated", "new content about dogs", &[], "note.md")
            .unwrap();
        assert_eq!(engine.search("cats", 10).unwrap().len(), 0);
        assert_eq!(engine.search("dogs", 10).unwrap().len(), 1);
    }

    #[test]
    fn multiple_notes_ranked() {
        let engine = test_engine();

        engine
            .index_note(
                Uuid::new_v4(),
                "Rust Guide",
                "A comprehensive guide to Rust programming language",
                &["rust".into()],
                "rust-guide.md",
            )
            .unwrap();
        engine
            .index_note(
                Uuid::new_v4(),
                "Python Guide",
                "Python is a dynamic language",
                &["python".into()],
                "python.md",
            )
            .unwrap();
        engine
            .index_note(
                Uuid::new_v4(),
                "Rust vs Go",
                "Comparing Rust and Go for systems programming",
                &["rust".into(), "go".into()],
                "rust-vs-go.md",
            )
            .unwrap();

        let results = engine.search("rust", 10).unwrap();
        assert_eq!(results.len(), 2);
        // Both rust-related notes should appear, python should not
        assert!(results.iter().all(|r| r.title != "Python Guide"));
    }
}

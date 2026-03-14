//! Integration tests for the Mneme API server.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::sync::RwLock;
use tower::ServiceExt;

use mneme_ai::DaimonClient;
use mneme_api::router::build_router;
use mneme_api::state::AppState;
use mneme_search::SearchEngine;
use mneme_store::Vault;

async fn test_app() -> (axum::Router, TempDir) {
    let dir = TempDir::new().unwrap();
    let vault = Vault::open(dir.path()).await.unwrap();
    let search = SearchEngine::in_memory().unwrap();
    let daimon = DaimonClient::new(None, None);
    let state = AppState {
        vault: Arc::new(RwLock::new(vault)),
        search: Arc::new(search),
        daimon: Arc::new(daimon),
    };
    (build_router(state), dir)
}

fn json_request(method: &str, uri: &str, body: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let body = body
        .map(|v| Body::from(serde_json::to_string(&v).unwrap()))
        .unwrap_or(Body::empty());
    builder.body(body).unwrap()
}

async fn response_json(resp: axum::http::Response<Body>) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn health_check() {
    let (app, _dir) = test_app().await;
    let resp = app
        .oneshot(json_request("GET", "/health", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["notes_count"], 0);
}

#[tokio::test]
async fn create_and_get_note() {
    let (app, _dir) = test_app().await;

    // Create
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/notes",
            Some(json!({
                "title": "Test Note",
                "content": "Hello world",
                "tags": ["test"]
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created = response_json(resp).await;
    let note_id = created["id"].as_str().unwrap();

    // Get
    let resp = app
        .oneshot(json_request("GET", &format!("/v1/notes/{note_id}"), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let fetched = response_json(resp).await;
    assert_eq!(fetched["title"], "Test Note");
    assert_eq!(fetched["content"], "Hello world");
}

#[tokio::test]
async fn list_notes() {
    let (app, _dir) = test_app().await;

    // Create 3 notes
    for i in 0..3 {
        app.clone()
            .oneshot(json_request(
                "POST",
                "/v1/notes",
                Some(json!({
                    "title": format!("Note {i}"),
                    "content": format!("Content {i}"),
                    "tags": []
                })),
            ))
            .await
            .unwrap();
    }

    let resp = app
        .oneshot(json_request("GET", "/v1/notes?limit=10", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let notes = response_json(resp).await;
    assert_eq!(notes.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn update_note() {
    let (app, _dir) = test_app().await;

    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/notes",
            Some(json!({
                "title": "Original",
                "content": "Old content",
                "tags": []
            })),
        ))
        .await
        .unwrap();
    let created = response_json(resp).await;
    let note_id = created["id"].as_str().unwrap();

    let resp = app
        .oneshot(json_request(
            "PUT",
            &format!("/v1/notes/{note_id}"),
            Some(json!({
                "title": "Updated",
                "content": "New content",
                "tags": ["updated"]
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let updated = response_json(resp).await;
    assert_eq!(updated["title"], "Updated");
}

#[tokio::test]
async fn delete_note() {
    let (app, _dir) = test_app().await;

    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/notes",
            Some(json!({
                "title": "To Delete",
                "content": "Bye",
                "tags": []
            })),
        ))
        .await
        .unwrap();
    let created = response_json(resp).await;
    let note_id = created["id"].as_str().unwrap();

    let resp = app
        .clone()
        .oneshot(json_request(
            "DELETE",
            &format!("/v1/notes/{note_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify gone
    let resp = app
        .oneshot(json_request("GET", &format!("/v1/notes/{note_id}"), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn search_notes() {
    let (app, _dir) = test_app().await;

    app.clone()
        .oneshot(json_request(
            "POST",
            "/v1/notes",
            Some(json!({
                "title": "Rust Programming",
                "content": "Rust is a systems programming language",
                "tags": ["rust"]
            })),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(json_request("GET", "/v1/search?q=rust", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let results = response_json(resp).await;
    assert!(!results.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn list_tags() {
    let (app, _dir) = test_app().await;

    app.clone()
        .oneshot(json_request(
            "POST",
            "/v1/notes",
            Some(json!({
                "title": "Tagged",
                "content": "Content",
                "tags": ["alpha", "beta"]
            })),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(json_request("GET", "/v1/tags", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let tags = response_json(resp).await;
    assert_eq!(tags.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn get_nonexistent_note() {
    let (app, _dir) = test_app().await;
    let fake_id = uuid::Uuid::new_v4();
    let resp = app
        .oneshot(json_request("GET", &format!("/v1/notes/{fake_id}"), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn rag_stats_without_daimon() {
    let (app, _dir) = test_app().await;
    let resp = app
        .oneshot(json_request("GET", "/v1/ai/rag/stats", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    assert_eq!(body["daimon_available"], false);
}

#[tokio::test]
async fn concept_extraction_endpoint() {
    let (app, _dir) = test_app().await;

    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/notes",
            Some(json!({
                "title": "Rust Systems",
                "content": "Rust is a systems programming language. Rust has a borrow checker. Rust ensures memory safety. The Rust compiler catches bugs at compile time.",
                "tags": ["rust"]
            })),
        ))
        .await
        .unwrap();
    let created = response_json(resp).await;
    let note_id = created["id"].as_str().unwrap();

    let resp = app
        .oneshot(json_request(
            "GET",
            &format!("/v1/ai/concepts/{note_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let concepts = response_json(resp).await;
    assert!(!concepts.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn suggest_tags_for_note() {
    let (app, _dir) = test_app().await;

    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/notes",
            Some(json!({
                "title": "Rust Programming Guide",
                "content": "Machine learning algorithms require training data for models. Neural networks are a subset of machine learning. Deep learning extends machine learning with multiple layers. Training models with machine learning produces intelligent systems. The machine learning pipeline includes feature engineering.",
                "tags": ["algorithms"]
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created = response_json(resp).await;
    let note_id = created["id"].as_str().unwrap();

    let resp = app
        .oneshot(json_request(
            "GET",
            &format!("/v1/ai/suggest-tags/{note_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let suggestions = response_json(resp).await;
    assert!(!suggestions.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn list_templates() {
    let (app, _dir) = test_app().await;

    let resp = app
        .oneshot(json_request("GET", "/v1/templates", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    let templates = body["templates"].as_array().unwrap();
    assert_eq!(templates.len(), 3);

    let names: Vec<&str> = templates
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"daily"));
    assert!(names.contains(&"meeting"));
    assert!(names.contains(&"project"));
}

#[tokio::test]
async fn render_template_without_create() {
    let (app, _dir) = test_app().await;

    let resp = app
        .oneshot(json_request(
            "POST",
            "/v1/templates/render",
            Some(json!({
                "template_name": "daily",
                "variables": {}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    assert!(!body["title"].as_str().unwrap().is_empty());
    assert!(!body["content"].as_str().unwrap().is_empty());
    assert_eq!(body["created"], false);
}

#[tokio::test]
async fn render_template_and_create_note() {
    let (app, _dir) = test_app().await;

    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/templates/render",
            Some(json!({
                "template_name": "daily",
                "variables": {},
                "create": true
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_json(resp).await;
    assert_eq!(body["created"], true);
    assert!(!body["title"].as_str().unwrap().is_empty());

    // Verify the note was actually created by listing notes
    let resp = app
        .oneshot(json_request("GET", "/v1/notes?limit=10", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let notes = response_json(resp).await;
    assert_eq!(notes.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn summarize_note_extractive() {
    let (app, _dir) = test_app().await;

    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/v1/notes",
            Some(json!({
                "title": "Detailed Topic",
                "content": "Rust is a modern systems programming language focused on safety and performance. The borrow checker enforces strict ownership rules at compile time. This eliminates entire classes of memory bugs without a garbage collector. Concurrency in Rust is fearless because data races are caught by the compiler. The type system provides zero-cost abstractions for building reliable software.",
                "tags": ["rust"]
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created = response_json(resp).await;
    let note_id = created["id"].as_str().unwrap();

    let resp = app
        .oneshot(json_request(
            "GET",
            &format!("/v1/ai/summarize/{note_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let summary = response_json(resp).await;
    assert!(!summary["summary"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn delete_tag() {
    let (app, _dir) = test_app().await;

    // Create a note with tags
    app.clone()
        .oneshot(json_request(
            "POST",
            "/v1/notes",
            Some(json!({
                "title": "Tagged Note",
                "content": "Some content",
                "tags": ["keep-me", "delete-me"]
            })),
        ))
        .await
        .unwrap();

    // List tags and find the one to delete
    let resp = app
        .clone()
        .oneshot(json_request("GET", "/v1/tags", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let tags = response_json(resp).await;
    let tags_arr = tags.as_array().unwrap();
    assert_eq!(tags_arr.len(), 2);

    let tag_to_delete = tags_arr
        .iter()
        .find(|t| t["name"] == "delete-me")
        .unwrap();
    let tag_id = tag_to_delete["id"].as_str().unwrap();

    // Delete the tag
    let resp = app
        .clone()
        .oneshot(json_request(
            "DELETE",
            &format!("/v1/tags/{tag_id}"),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify only one tag remains
    let resp = app
        .oneshot(json_request("GET", "/v1/tags", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let tags = response_json(resp).await;
    let remaining = tags.as_array().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0]["name"], "keep-me");
}

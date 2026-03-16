//! AI-powered HTTP request handlers.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mneme_ai::concepts::{self, Concept};
use mneme_ai::linker::{AutoLinker, LinkSuggestion};
use mneme_ai::rag::{RagAnswer, RagPipeline};
use mneme_ai::summarizer::{NoteSummary, Summarizer};

use crate::handlers::ErrorResponse;
use crate::state::AppState;

// --- Request/Response types ---

#[derive(Deserialize)]
pub struct RagQueryParams {
    pub q: String,
    pub top_k: Option<usize>,
}

#[derive(Deserialize)]
pub struct SuggestLinksParams {
    pub top_k: Option<usize>,
}

#[derive(Serialize)]
pub struct IngestResponse {
    pub note_id: Uuid,
    pub chunks_ingested: usize,
}

#[derive(Serialize)]
pub struct RagStatsResponse {
    pub index_size: usize,
    pub daimon_available: bool,
    pub local_vectors: usize,
    pub local_available: bool,
    pub eval: mneme_ai::rag_eval::RagEvalAggregates,
}

/// Helper to get the active vault.
macro_rules! active_vault {
    ($mgr:expr) => {{
        $mgr.active().ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "No active vault".into(),
                }),
            )
        })?
    }};
}

// --- RAG ---

/// Ask a question across all notes (RAG query).
pub async fn rag_query(
    State(state): State<AppState>,
    Query(params): Query<RagQueryParams>,
) -> Result<Json<RagAnswer>, (StatusCode, Json<ErrorResponse>)> {
    let pipeline = RagPipeline::new((*state.daimon).clone());
    let answer = pipeline.query(&params.q, params.top_k).await.map_err(|e| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    // Record eval scores in aggregates
    if let Some(ref eval) = answer.eval {
        let mut vs = state.vaults.write().await;
        if let Some(eng) = vs.active_engines_mut() {
            eng.rag_eval.record(eval);
        }
    }

    Ok(Json(answer))
}

/// Ingest a specific note into the RAG pipeline.
pub async fn rag_ingest_note(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<IngestResponse>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let ov = active_vault!(vs);

    let note = ov.vault.vault.get_note(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    // Also index locally
    let _ = ov
        .semantic()
        .index_note(id, &note.note.title, &note.content);

    let pipeline = RagPipeline::new((*state.daimon).clone());
    let chunks = pipeline
        .ingest_note(id, &note.note.title, &note.content)
        .await
        .map_err(|e| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(IngestResponse {
        note_id: id,
        chunks_ingested: chunks,
    }))
}

/// Get RAG pipeline stats.
pub async fn rag_stats(
    State(state): State<AppState>,
) -> Result<Json<RagStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;

    let (local_vectors, local_available, eval) = match vs.active() {
        Some(ov) => (
            ov.semantic().vector_count(),
            ov.semantic().is_available(),
            ov.engines.rag_eval.clone(),
        ),
        None => (0, false, mneme_ai::rag_eval::RagEvalAggregates::default()),
    };

    let pipeline = RagPipeline::new((*state.daimon).clone());
    match pipeline.stats().await {
        Ok(size) => Ok(Json(RagStatsResponse {
            index_size: size,
            daimon_available: true,
            local_vectors,
            local_available,
            eval,
        })),
        Err(_) => Ok(Json(RagStatsResponse {
            index_size: 0,
            daimon_available: false,
            local_vectors,
            local_available,
            eval,
        })),
    }
}

// --- Summarization ---

/// Summarize a note's content.
pub async fn summarize_note(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<NoteSummary>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let ov = active_vault!(vs);

    let note = ov.vault.vault.get_note(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let summarizer = Summarizer::new((*state.daimon).clone());
    let summary = summarizer.summarize(&note.content).await.map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    Ok(Json(summary))
}

// --- Auto-linking ---

/// Suggest links for a note based on content similarity.
pub async fn suggest_links(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<SuggestLinksParams>,
) -> Result<Json<Vec<LinkSuggestion>>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let ov = active_vault!(vs);

    let note = ov.vault.vault.get_note(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let linker = AutoLinker::new((*state.daimon).clone());
    let top_k = params.top_k.unwrap_or(5);
    let suggestions = linker
        .suggest_links(id, &note.note.title, &note.content, top_k)
        .await
        .map_err(|e| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;
    Ok(Json(suggestions))
}

// --- Concept extraction ---

/// Extract concepts from a note's content.
pub async fn extract_concepts(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Concept>>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let ov = active_vault!(vs);

    let note = ov.vault.vault.get_note(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let extracted = concepts::extract_concepts(&note.content).map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    Ok(Json(extracted))
}

// --- AI Writing ---

#[derive(Deserialize)]
pub struct WriteAssistRequest {
    pub action: mneme_ai::writer::WriteAction,
    pub text: String,
    pub context: Option<String>,
}

/// AI writing assistance (complete, reword, expand).
pub async fn write_assist(
    State(state): State<AppState>,
    Json(req): Json<WriteAssistRequest>,
) -> Result<Json<mneme_ai::writer::WriteResult>, (StatusCode, Json<ErrorResponse>)> {
    let writer = mneme_ai::writer::Writer::new((*state.daimon).clone());
    let write_req = mneme_ai::writer::WriteRequest {
        action: req.action,
        text: req.text,
        context: req.context,
    };
    let result = writer.assist(&write_req).await.map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    Ok(Json(result))
}

// --- Translation ---

/// Translate a note's content.
pub async fn translate_note(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<TranslateParams>,
) -> Result<Json<mneme_ai::translator::TranslateResult>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let ov = active_vault!(vs);

    let note = ov.vault.vault.get_note(id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let translator = mneme_ai::translator::Translator::new((*state.daimon).clone());
    let req = mneme_ai::translator::TranslateRequest {
        content: note.content,
        target_language: params.lang.clone(),
        source_language: params.source_lang.clone(),
        preserve_formatting: true,
    };
    let result = translator.translate(&req).await.map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct TranslateParams {
    pub lang: String,
    pub source_lang: Option<String>,
}

/// List supported languages.
pub async fn list_languages() -> Json<Vec<mneme_ai::translator::Language>> {
    Json(mneme_ai::translator::Translator::supported_languages())
}

// --- Temporal Analysis ---

/// Temporal analysis across all notes.
pub async fn temporal_analysis(
    State(state): State<AppState>,
) -> Result<Json<mneme_ai::temporal::TemporalReport>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let ov = active_vault!(vs);

    let notes = ov.vault.vault.list_notes(1000, 0).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let mut snapshots = Vec::new();
    for note in &notes {
        if let Ok(full) = ov.vault.vault.get_note(note.id).await {
            snapshots.push(mneme_ai::temporal::NoteSnapshot {
                title: full.note.title,
                content: full.content,
                tags: full.tags,
                created_at: full.note.created_at,
                updated_at: full.note.updated_at,
            });
        }
    }

    let report = mneme_ai::temporal::analyze_temporal(&snapshots).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    Ok(Json(report))
}

// --- Clustering ---

#[derive(Deserialize)]
pub struct ClusterParams {
    /// Number of clusters. If omitted, uses elbow heuristic.
    pub k: Option<usize>,
    /// Maximum k to try when using the elbow heuristic (default: 8).
    pub max_k: Option<usize>,
    /// Whether to request LLM-generated labels from daimon (default: false).
    pub label: Option<bool>,
}

/// Cluster notes by embedding similarity.
pub async fn cluster_notes(
    State(state): State<AppState>,
    Query(params): Query<ClusterParams>,
) -> Result<Json<mneme_ai::clustering::ClusteringResult>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let ov = active_vault!(vs);

    let max_k = params.max_k.unwrap_or(8);

    // Load all notes
    let notes = ov.vault.vault.list_notes(1000, 0).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    if notes.is_empty() {
        return Ok(Json(mneme_ai::clustering::ClusteringResult {
            k: 0,
            total_notes: 0,
            total_inertia: 0.0,
            clusters: vec![],
        }));
    }

    // Embed each note
    let semantic = ov.semantic();
    let mut note_embeddings = Vec::new();

    for note in &notes {
        let text = format!("{}\n", note.title);
        if let Ok(Some(emb)) = semantic.embed(&text) {
            note_embeddings.push(mneme_ai::clustering::NoteEmbedding {
                id: note.id,
                title: note.title.clone(),
                embedding: emb,
            });
        }
    }

    if note_embeddings.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "Semantic engine unavailable — cannot embed notes for clustering".into(),
            }),
        ));
    }

    let mut result = mneme_ai::clustering::cluster_notes(&note_embeddings, params.k, max_k);

    // Optionally label clusters via LLM
    if params.label.unwrap_or(false) {
        for cluster in &mut result.clusters {
            if let Ok(resp) = state.daimon.label_cluster(&cluster.note_titles).await {
                cluster.label = resp.label;
                cluster.summary = resp.summary;
            }
        }
    }

    Ok(Json(result))
}

// --- Knowledge QA ---

/// Run knowledge quality assertions against the active vault.
pub async fn run_qa(
    State(state): State<AppState>,
) -> Result<Json<mneme_ai::qa_bridge::QaRunResult>, (StatusCode, Json<ErrorResponse>)> {
    let vs = state.vaults.read().await;
    let ov = active_vault!(vs);

    let notes = ov.vault.vault.list_notes(1000, 0).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e.to_string() }),
        )
    })?;

    // Build note metadata for assertion generation
    let mut note_meta = Vec::new();
    for note in &notes {
        let backlinks = ov.vault.vault.db().get_backlinks(note.id).await.unwrap_or_default();
        let note_tags = ov.vault.vault.db().get_note_tags(note.id).await.unwrap_or_default();
        note_meta.push((note.id, note.title.clone(), note_tags, backlinks.len()));
    }

    let tag_counts: Vec<(String, usize)> = {
        let mut counts = std::collections::HashMap::new();
        for (_, _, note_tags, _) in &note_meta {
            for tag in note_tags {
                *counts.entry(tag.clone()).or_insert(0usize) += 1;
            }
        }
        counts.into_iter().collect()
    };

    let assertions = mneme_ai::qa_bridge::generate_assertions(&note_meta, &tag_counts);

    // If Agnostic is available, submit the suite
    if state.qa_client.is_available().await {
        let vault_name = ov.vault.info.name.clone();
        drop(vs);
        match state.qa_client.run_assertions(&assertions, &vault_name).await {
            Ok(run_id) => {
                match state.qa_client.get_run_result(&run_id).await {
                    Ok(result) => return Ok(Json(result)),
                    Err(_) => {
                        return Ok(Json(mneme_ai::qa_bridge::QaRunResult {
                            run_id,
                            status: "running".into(),
                            total_assertions: assertions.len(),
                            passed: 0,
                            failed: 0,
                            failures: vec![],
                        }));
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Agnostic QA submit failed: {e}");
            }
        }
    } else {
        drop(vs);
    }

    // Fallback: run assertions locally
    let failed: Vec<mneme_ai::qa_bridge::QaFailure> = assertions
        .iter()
        .map(|a| mneme_ai::qa_bridge::QaFailure {
            assertion: a.description.clone(),
            expected: a.expected.clone(),
            actual: "not met".into(),
        })
        .collect();
    let failed_count = failed.len();

    Ok(Json(mneme_ai::qa_bridge::QaRunResult {
        run_id: "local".into(),
        status: "completed".into(),
        total_assertions: assertions.len(),
        passed: assertions.len() - failed_count,
        failed: failed_count,
        failures: failed,
    }))
}

// --- Structured Query ---

#[derive(Deserialize)]
pub struct StructuredSearchParams {
    pub q: String,
}

/// Parse a structured query (DSL) and return the parsed components.
pub async fn parse_search_query(
    Query(params): Query<StructuredSearchParams>,
) -> Json<mneme_search::query_dsl::StructuredQuery> {
    Json(mneme_search::query_dsl::parse_query(&params.q))
}

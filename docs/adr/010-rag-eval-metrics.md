# ADR-010: RAG Evaluation Metrics

## Status
Accepted

## Context

RAG pipelines can return plausible-sounding but unfaithful answers. Without quality signals, users have no way to gauge how much to trust a response, and operators cannot detect degradation over time. Most RAG evaluation frameworks (RAGAS, DeepEval) rely on LLM-as-judge calls, which add latency, cost, and a hard dependency on an external model.

## Decision

### Token-Overlap Scoring
`mneme-ai/src/rag_eval.rs` implements three local-only metrics using token overlap — no LLM required:

- **Faithfulness** (0.0–1.0): Fraction of answer tokens that appear in the retrieved context. Measures whether the answer is grounded in the source material.
- **Answer relevance** (0.0–1.0): Fraction of answer tokens that appear in the original query. Measures whether the answer addresses the question asked.
- **Chunk utilization** (0.0–1.0): Fraction of context tokens that appear in the answer. Measures how much of the retrieved material was actually used.

A simple tokenizer with stopword filtering removes noise words before comparison.

### Weighted Overall Score
An overall score combines the three metrics as a weighted average: 50% faithfulness, 30% relevance, 20% utilization. Faithfulness is weighted highest because grounding is the most critical property — an unfaithful answer is worse than an irrelevant one.

### Per-Query and Aggregate Reporting
Each `RagAnswer` now carries an optional `eval: RagEvalScores` field, computed automatically by the RAG query handler. `RagEvalAggregates` tracks running averages per vault, exposed via `/v1/ai/rag/stats`. This lets operators monitor vault-level RAG health without inspecting individual queries.

## Consequences

- **Positive**: Zero additional latency — scoring is a simple set-intersection computation on tokens already in memory
- **Positive**: No LLM dependency — works fully offline, deterministic, and reproducible
- **Positive**: Vault-level aggregates surface degradation trends (e.g., after bulk import of low-quality notes)
- **Negative**: Token overlap is a coarse proxy — it cannot detect semantic paraphrasing or subtle hallucination
- **Negative**: Stopword list is English-only; multilingual vaults may see noisier scores
- **Trade-off**: If higher-fidelity evaluation is needed in the future, an LLM-as-judge layer can be added on top without replacing the local baseline

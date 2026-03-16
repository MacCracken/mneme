# ADR-008: Retrieval Optimizer

## Status
Accepted

## Context

Mneme's hybrid search combines full-text (BM25) and semantic (vector) signals with a recency boost. The optimal blend weights vary by user, vault content, and query type. Static weights cannot adapt to these differences. We needed a lightweight mechanism to learn better weights from user behavior.

## Decision

### Thompson Sampling Bandit
Implement a multi-armed bandit in `mneme-search/src/retrieval_optimizer.rs` using Thompson Sampling with Beta-distributed arms. Four arms represent different blend strategies:

1. **balanced** — equal weight to full-text and semantic
2. **fulltext_heavy** — biased toward BM25
3. **semantic_heavy** — biased toward vector similarity
4. **recency_boost** — strong recency bias

Each search selects an arm by sampling from its Beta(alpha, beta) distribution. The selected arm's `BlendWeights` are passed to `weighted_hybrid_merge`.

### Feedback Loop
Search responses include a `search_id`. When the user clicks a result, the `/v1/search/feedback` endpoint (or MCP `mneme_search_feedback` tool) records the signal. Clicks increment the selected arm's alpha (success); searches without clicks increment beta (failure) on a timeout.

### Persistence
Optimizer state (arm parameters, selection counts) persists in `.mneme/optimizer.json`. On cold start, arms initialize with weak priors (alpha=1, beta=1) so the bandit explores broadly before converging.

## Consequences

- **Positive**: Search quality improves automatically with use — no manual tuning
- **Positive**: Thompson Sampling is simple, well-understood, and computationally trivial
- **Positive**: Four arms are enough to capture the major ranking strategies without combinatorial explosion
- **Negative**: Requires user feedback to learn — passive users see no improvement
- **Negative**: Cold start explores randomly for the first ~50 searches before converging

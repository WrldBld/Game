# ADR-004: VCR-Based E2E Testing for LLM Integration

## Status

Accepted

## Date

2026-01-13

## Context

WrldBldr uses LLMs (Ollama) for NPC dialogue, narrative generation, and challenge suggestions. Testing LLM-dependent flows presents challenges:

1. **Non-determinism**: LLM responses vary between runs
2. **Cost**: Each test run consumes compute resources
3. **Speed**: LLM calls add seconds per request
4. **Availability**: Tests fail if Ollama is down
5. **Reproducibility**: Cannot debug issues without consistent responses

## Decision

Implement a **VCR (Video Cassette Recorder) pattern** for LLM testing:

1. **Recording**: During development, capture real LLM responses to "cassette" files
2. **Playback**: During CI/test runs, replay recorded responses
3. **Live mode**: Optionally call real LLM for manual testing

Three modes controlled by `E2E_LLM_MODE` environment variable:
- `record`: Call real Ollama, save responses to cassettes
- `playback` (default): Load responses from cassettes
- `live`: Always call real Ollama, no recording

## Consequences

### Positive

- **Deterministic tests**: Same responses every run
- **Fast execution**: Playback is milliseconds vs seconds
- **CI-friendly**: No Ollama required in CI
- **Cost-effective**: No compute for test runs
- **Debuggable**: Can inspect exact prompts and responses in cassette files

### Negative

- **Cassette maintenance**: Must re-record when prompts change
- **Sequential playback**: Current implementation uses order-based matching (see future work)
- **Staleness risk**: Cassettes may not reflect current LLM behavior
- **Storage overhead**: Cassette files can be large

### Neutral

- Developers must remember to record when changing LLM interactions
- Cassettes are JSON files committed to repo

## Implementation

```rust
// Create VCR LLM (auto-detects mode from env)
let llm = create_e2e_llm("test_name");

// Use in test context
let ctx = E2ETestContext::setup_with_llm(llm).await?;
```

Cassette format:
```json
{
  "version": "1.0",
  "llm_model": "llama3.2:latest",
  "recordings": [
    {
      "request_summary": "System: You are Marta...",
      "response": { "content": "Hello, traveler!", ... }
    }
  ]
}
```

## Future Work

- **Content-based fingerprinting**: Match requests by content hash instead of order
- **Fuzzy matching**: Allow minor prompt variations to match same recording
- **Cassette compression**: Reduce storage for large test suites

## Alternatives Considered

### 1. Mock LLM with Fixed Responses

Hardcode test responses without recording real LLM output.

**Rejected:** Responses wouldn't be realistic. Tests might pass but not reflect actual system behavior.

### 2. Always Use Real LLM

Run all tests against live Ollama.

**Rejected:** Too slow for CI, non-deterministic, requires Ollama availability.

### 3. Snapshot Testing

Snapshot entire test outputs including LLM responses.

**Rejected:** Doesn't provide request-level control. Can't inspect individual prompts easily.

## References

- [e2e-testing.md](e2e-testing.md) - Full E2E testing documentation
- VCR pattern: https://github.com/vcr/vcr (Ruby implementation that inspired this)

# ADR-006: LLM Port Trait Design

## Status

Accepted

## Date

2026-01-13

## Context

WrldBldr uses LLMs for:
- NPC dialogue generation
- Narrative event suggestions
- Challenge outcome descriptions
- Custom condition evaluation
- Staging suggestions (NPC presence reasoning)

Need to abstract LLM access to:
1. Enable swapping providers (Ollama, Claude, OpenAI)
2. Support testing with mocks/VCR
3. Handle tool calling for game mechanics
4. Manage context windows and token limits

## Decision

Define a minimal **`LlmPort` trait** with one method:

```rust
#[async_trait]
pub trait LlmPort: Send + Sync {
    /// Generate a response from the LLM.
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;
}
```

Supporting types:
- `LlmRequest`: system prompt, messages, temperature, max_tokens
- `LlmResponse`: content, finish_reason, usage
- `ChatMessage`: role (user/assistant/system), content

### Tool Calling via XML

Instead of OpenAI-style function calling, we use **XML-based tool extraction**:
- Prompts instruct the LLM to emit `<tool name="...">{"param": "value"}</tool>` tags
- `response_parser.rs` extracts these tags via regex
- Tools flow through the DM approval queue before execution

This approach:
- Works with any LLM (no provider-specific function calling)
- Keeps all game logic in the content (parseable, loggable)
- Enables consistent VCR recording/playback

## Consequences

### Positive

- Simple interface (1 method covers all use cases)
- Easy to mock for unit tests
- VCR can wrap the trait seamlessly
- Provider-agnostic (works with any OpenAI-compatible API)
- XML tool extraction works with any model

### Negative

- No streaming support (responses are complete)
- No built-in retry/resilience (handled by wrapper)
- Token counting is provider-specific
- Tool extraction depends on LLM following XML format

### Neutral

- Callers must construct `LlmRequest` manually
- Tool extraction is done post-response via `response_parser`

## Design Decisions

### Single Request Object vs Builder Pattern

Chose request object because:
- All parameters visible at call site
- Easy to log and debug
- Can be serialized for VCR recording

### XML Tools vs Function Calling

Chose XML-based tool extraction because:
- Works with any LLM (no provider-specific APIs)
- All response content in one place (easier to log/debug)
- VCR cassettes capture the complete response
- DM can see exactly what the LLM proposed

### No Streaming

Chose complete responses because:
- Simpler implementation
- TTRPG doesn't need real-time streaming (responses are short)
- VCR recording is simpler with complete responses
- Can add streaming later if needed

## Implementation Notes

Current implementations:
- `OpenAICompatibleClient`: Production implementation for any OpenAI-compatible API (Ollama, MLX, vLLM, etc.)
- `VcrLlm`: Test wrapper for record/playback
- `ResilientLlm`: Wrapper adding retry logic
- `NoopLlm`: Test implementation returning errors

Example usage:
```rust
let request = LlmRequest::new(vec![
    ChatMessage::user("What do you see in the tavern?"),
])
.with_system_prompt("You are Marta, the innkeeper.")
.with_temperature(0.7)
.with_max_tokens(Some(500));

let response = llm.generate(request).await?;
```

## Alternatives Considered

### 1. Provider-Specific Traits

Separate traits for each provider (OllamaPort, ClaudePort).

**Rejected:** Would require different code paths per provider. Abstraction allows swapping without code changes.

### 2. Generic Chat Completions Interface

Mirror OpenAI's API directly.

**Rejected:** Couples to specific provider's API shape. Our minimal interface is easier to adapt.

### 3. Message Bus / Queue Based

Async requests through message queue.

**Rejected:** Adds complexity for synchronous use case. Queue-based async can be built on top if needed.

## Future Work

- Streaming support for long-form generation
- Token counting abstraction
- Automatic context window management
- Rate limiting / quota tracking

## References

- `crates/engine/src/infrastructure/ports.rs` - Trait definition
- `crates/engine/src/infrastructure/openai_compatible.rs` - OpenAI-compatible client
- `crates/engine/src/infrastructure/resilient_llm.rs` - Resilience wrapper
- `crates/engine/src/use_cases/queues/response_parser.rs` - XML tool extraction

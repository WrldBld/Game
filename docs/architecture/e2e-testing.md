# E2E Testing Infrastructure

End-to-end testing infrastructure for validating complete gameplay flows with real Neo4j database and VCR-recorded LLM responses.

## Overview

E2E tests exercise the full application stack:
- Real Neo4j testcontainer
- Complete App with all use cases wired
- VCR LLM recording/playback for deterministic tests
- Comprehensive event logging for analysis

## Architecture

```
e2e_tests/
├── mod.rs                 # Module exports
├── neo4j_test_harness.rs  # Neo4j testcontainer management
├── vcr_llm.rs             # LLM recording/playback
├── event_log.rs           # Comprehensive event logging
├── logging_queue.rs       # Queue wrapper for event capture
├── e2e_helpers.rs         # Test context and utilities
├── gameplay_flow_tests.rs # Flow-based tests
├── gameplay_loop_tests.rs # World structure tests
├── cassettes/             # VCR recordings (JSON)
└── logs/                  # Event log output (JSON)
```

## Running Tests

```bash
# Playback mode (default) - uses recorded cassettes
cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored --test-threads=1

# Record mode - call real Ollama, save responses
E2E_LLM_MODE=record cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored --test-threads=1

# Live mode - always call real LLM
E2E_LLM_MODE=live cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored --test-threads=1
```

## VCR LLM System

The VCR LLM wraps the real LLM client to record and playback responses:

```rust
// Create VCR LLM (auto-detects mode from E2E_LLM_MODE env var)
let llm = create_e2e_llm("test_name");

// Modes:
// - Record: Call real Ollama, save to cassette
// - Playback: Load responses from cassette (default)
// - Live: Always call real Ollama, no recording
```

### Cassette Format

```json
{
  "version": "1.0",
  "recorded_at": "2026-01-12T10:00:00Z",
  "llm_model": "llama3.2:latest",
  "recordings": [
    {
      "index": 0,
      "request_summary": "System: You are Marta Heath...",
      "response": {
        "content": "Good morning, traveler!...",
        "tool_calls": [],
        "finish_reason": "Stop"
      },
      "recorded_at": "2026-01-12T10:00:01Z"
    }
  ]
}
```

## Event Logging System

Comprehensive logging captures all events, prompts, and responses for analysis.

### Enabling Event Logging

```rust
// Create event log
let event_log = create_shared_log("test_name");

// Create VCR LLM with event logging
let llm = {
    let vcr = VcrLlm::from_env(cassette_path);
    Arc::new(vcr.with_event_log(event_log.clone()))
};

// Create context with logging
let ctx = E2ETestContext::setup_with_llm_and_logging(llm, event_log.clone())
    .await?;

// Run test...

// Finalize and save
ctx.finalize_event_log(TestOutcome::Pass);
ctx.save_event_log(&E2ETestContext::default_log_path("test_name"))?;
```

### Event Types

| Category | Events |
|----------|--------|
| Session | `SessionStarted`, `SessionEnded` |
| Queue | `ActionEnqueued`, `ActionProcessed`, `LlmRequestEnqueued`, `LlmRequestProcessed`, `ApprovalEnqueued` |
| LLM | `LlmPromptSent`, `LlmResponseReceived` (full prompts/responses) |
| Conversation | `ConversationStarted`, `ConversationTurn`, `ConversationEnded` |
| Challenge | `ChallengeTriggered`, `ChallengeRoll`, `ChallengeOutcome` |
| Staging | `StagingRequired`, `StagingApproved` |
| Time | `TimeAdvanced`, `TimePaused` |
| Narrative | `NarrativeEventTriggered` |
| Approval | `ApprovalDecision` |
| Error | `Error` |

### Event Log Format

```json
{
  "version": "1.0",
  "test_name": "test_full_gameplay_session",
  "started_at": "2026-01-12T10:00:00Z",
  "ended_at": "2026-01-12T10:02:30Z",
  "world_id": "abc123",
  "outcome": "pass",
  "events": [
    {
      "timestamp": "2026-01-12T10:00:01Z",
      "event": {
        "type": "ConversationStarted",
        "id": "conv-123",
        "pc_id": "pc-456",
        "npc_id": "npc-789",
        "npc_name": "Marta Hearthwood"
      }
    },
    {
      "timestamp": "2026-01-12T10:00:02Z",
      "event": {
        "type": "LlmPromptSent",
        "request_id": "req-001",
        "system_prompt": "You are Marta Hearthwood...",
        "messages": [{ "role": "user", "content": "Good morning!", "truncated": false }],
        "temperature": 0.7,
        "max_tokens": 500,
        "tools": ["trigger_challenge"]
      }
    }
  ],
  "summary": {
    "event_counts": { "LlmPromptSent": 2, "ConversationStarted": 1 },
    "llm_calls": 2,
    "total_tokens": { "prompt": 520, "completion": 180, "total": 700 },
    "avg_llm_latency_ms": 2750.0,
    "conversations_count": 1,
    "challenges_count": 0,
    "errors_count": 0
  }
}
```

### Analyzing Logs

```bash
# View summary
cat src/e2e_tests/logs/test_full_gameplay_session.json | jq '.summary'

# View all LLM prompts
cat src/e2e_tests/logs/test_full_gameplay_session.json | jq '.events[] | select(.event.type == "LlmPromptSent")'

# View conversation turns
cat src/e2e_tests/logs/test_full_gameplay_session.json | jq '.events[] | select(.event.type == "ConversationTurn")'
```

## Test Context

`E2ETestContext` provides:
- Real Neo4j testcontainer
- Seeded Thornhaven test world
- Full App stack with all use cases
- Fixed clock for deterministic time
- Optional event logging

```rust
// Basic setup (no LLM)
let ctx = E2ETestContext::setup().await?;

// With VCR LLM
let llm = create_e2e_llm("test_name");
let ctx = E2ETestContext::setup_with_llm(llm).await?;

// With LLM and event logging
let event_log = create_shared_log("test_name");
let llm = Arc::new(VcrLlm::from_env(path).with_event_log(event_log.clone()));
let ctx = E2ETestContext::setup_with_llm_and_logging(llm, event_log).await?;
```

## Helper Functions

| Function | Purpose |
|----------|---------|
| `create_test_player()` | Create PC directly in Neo4j |
| `create_player_character_via_use_case()` | Create PC via management use case |
| `approve_staging_with_npc()` | Stage an NPC in a region |
| `start_conversation_with_npc()` | Start conversation and process to approval |
| `run_conversation_turn()` | Continue conversation turn |

## Test World: Thornhaven

Tests use a seeded world with:
- **World**: Thornhaven Village
- **Location**: Drowsy Dragon Inn
- **Regions**: Common Room, Cellar, Kitchen, Upstairs Hallway
- **NPCs**: Marta Hearthwood (innkeeper), Torvin Ashwood, others
- **Acts/Scenes**: Act 1 setup, Scene 1 arrival
- **Challenges**: Persuasion, Perception checks
- **Narrative Events**: Inn arrival event

## Neo4j Container Configuration

Testcontainers configuration for reliability:
- Version: `neo4j:5.26.0-community` (pinned)
- Memory: 256m-512m heap, 128m pagecache
- Wait strategy: 5-second initial wait + exponential backoff retry
- Connection verification: `RETURN 1` query test

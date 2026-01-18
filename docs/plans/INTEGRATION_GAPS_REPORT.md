# Integration Gaps Report

**Date:** January 18, 2026  
**Purpose:** Comprehensive analysis of integration gaps between WrldBldr systems

---

## Executive Summary

This report identifies **critical integration gaps** between WrldBldr's game systems that prevent the product vision from being fully realized. The gaps fall into three categories:

1. **LLM Response Parsing** - Structured output from LLM not being extracted
2. **Tool Execution** - Approved tools not being automatically executed
3. **Context Population** - LLM missing critical world context

| Category | Critical Gaps | Impact |
|----------|---------------|--------|
| Dialogue-Narrative | 2 | LLM cannot suggest narrative events |
| Challenge-Character | 2 | Stats not broadcast, modifiers show 0 |
| Inventory-Dialogue | 3 | Approved give_item tools don't execute |
| Staging-Scene | 2 | Visual state not applied to scenes |
| Observation-Dialogue | 2 | LLM doesn't know PC's known NPCs |
| Lore-Dialogue | 4 | Lore system not integrated with dialogue |

---

## Critical Gap #1: LLM Response Parsing Not Implemented

### Affected Systems
- Dialogue System
- Narrative System  
- Challenge System

### The Problem

The LLM is instructed (via prompt templates) to output structured XML:
```xml
<reasoning>Internal thoughts...</reasoning>
<dialogue>What the NPC says</dialogue>
<challenge_suggestion>{"challenge_id": "...", "confidence": "high"}</challenge_suggestion>
<narrative_event_suggestion>{"event_id": "...", "confidence": "high"}</narrative_event_suggestion>
```

**But no parser exists to extract these.** In `use_cases/queues/mod.rs`:
```rust
let approval_data = ApprovalRequestData {
    internal_reasoning: String::new(),      // Always empty
    challenge_suggestion: None,              // Always None
    narrative_event_suggestion: None,        // Always None
    // ...
};
```

### Impact
- DM never sees LLM's reasoning
- LLM cannot suggest challenges during dialogue
- LLM cannot suggest narrative event triggers
- All structured suggestions are lost

### Fix Required
Create `use_cases/queues/response_parser.rs` that:
1. Extracts `<reasoning>` content
2. Extracts `<challenge_suggestion>` JSON
3. Extracts `<narrative_event_suggestion>` JSON
4. Strips tags from dialogue text

---

## Critical Gap #2: Approved Tools Not Executed

### Affected Systems
- Inventory System
- Dialogue System
- Character System

### The Problem

When DM approves tools (GiveItem, ChangeRelationship, etc.), they are NOT executed.

In `use_cases/approval/mod.rs`:
```rust
// Returns approved tool IDs, but doesn't execute them
Ok(ApprovalResult {
    approved_tools: result.approved_tools.clone(), // Just IDs!
    // ...
})
```

The E2E test even has a workaround comment:
```rust
// If give_item tools were approved, manually simulate the item transfer
// (The full effect execution happens in the approval flow)
for tool in give_item_tools {
    // ... manually calls inventory.give_item.execute()
}
```

### Impact
- Items approved via dialogue don't transfer
- Relationship changes don't apply
- All dialogue tool suggestions are cosmetic only

### Fix Required
Create `use_cases/approval/tool_executor.rs` that:
1. Takes `approved_tools: Vec<String>` IDs
2. Looks up original `ProposedTool` structs from pending approval
3. Dispatches to appropriate use case:
   - `give_item` → `InventoryUseCases::give_item`
   - `change_relationship` → `CharacterRepo::update_relationship`
   - etc.

---

## Critical Gap #3: Active Narrative Events Not in LLM Context

### Affected Systems
- Narrative System
- Dialogue System

### The Problem

`GamePromptRequest` has an `active_narrative_events` field, but it's always empty:
```rust
Ok(GamePromptRequest {
    active_narrative_events: vec![], // Always empty!
    // ...
})
```

### Impact
- LLM doesn't know about available narrative events
- Cannot make informed trigger suggestions
- Misses story beats that should fire based on context

### Fix Required
In `use_cases/queues/mod.rs` `build_prompt()`:
```rust
let active_narrative_events = self.narrative
    .list_active_untriggered(world_id).await?
    .into_iter()
    .map(|e| ActiveNarrativeEventContext {
        id: e.id().to_string(),
        name: e.name().to_string(),
        trigger_hints: extract_trigger_hints(&e),
        // ...
    })
    .collect();
```

---

## Critical Gap #4: CharacterStatUpdated Never Broadcast

### Affected Systems
- Challenge System
- Character System

### The Problem

When `ModifyCharacterStat` trigger executes, stats are updated in DB but:
- `ServerMessage::CharacterStatUpdated` is never sent
- Player UI doesn't know stats changed

```rust
// In challenge/mod.rs - modifies stat but doesn't broadcast
OutcomeTrigger::ModifyCharacterStat { stat, modifier } => {
    self.pc.modify_stat(target_pc_id, stat, *modifier).await?;
    Ok(()) // No broadcast!
}
```

### Impact
- Players see stale character sheets after challenge outcomes
- Must manually refresh to see stat changes

### Fix Required
After `modify_stat()`:
```rust
broadcast_to_world(world_id, ServerMessage::CharacterStatUpdated {
    pc_id: target_pc_id.to_string(),
    stat_name: stat.clone(),
    new_value: new_value,
    change: *modifier,
}).await;
```

---

## Critical Gap #5: TriggerChallengePrompt Shows 0 Modifier

### Affected Systems
- Challenge System
- Character System

### The Problem

When DM triggers a challenge, prompt data always shows:
```rust
Ok(ChallengePromptData {
    skill_name: String::new(),      // Empty!
    character_modifier: 0,           // Always 0!
    // ...
})
```

### Impact
- Player sees "Modifier: +0" in challenge prompt
- Doesn't know their actual bonus before rolling
- Confusing UX (actual roll uses correct modifier)

### Fix Required
Add `target_pc_id` parameter and fetch modifier:
```rust
let modifier = if let Some(stat) = challenge.check_stat() {
    pc.sheet_data()
        .and_then(|s| s.get_numeric_value(stat))
        .unwrap_or(0)
} else { 0 };
```

---

## Critical Gap #6: Visual State Not Applied After Staging

### Affected Systems
- Staging System
- Scene System
- Visual State System

### The Problem

1. **SceneChanged not sent after StagingReady**: When DM approves staging, only `StagingReady` is sent, not `SceneChanged`
2. **Visual state ignored in player UI**: The handler has a TODO comment:
   ```rust
   .. // visual_state - TODO: Handle in UI when implemented
   ```
3. **Backdrop overrides not in SceneChanged**: Only base `backdrop_asset` included

### Impact
- Players waiting for staging don't get full scene data
- Active location/region states don't change backdrops
- Visual state system is wired server-side but not applied client-side

### Fix Required
1. Send `SceneChanged` after `StagingReady` approval
2. Include visual state backdrop override in `RegionInfo`
3. Apply visual state in player session handler

---

## Critical Gap #7: Lore System Not Integrated with Dialogue

### Affected Systems
- Lore System
- Dialogue System

### The Problem

1. **No `discover_lore` tool**: Despite documentation spec, tool not implemented
2. **NPC lore knowledge not in context**: LLM doesn't know what lore NPCs have
3. **No lore execution**: Even if tool existed, no code to grant lore

### Impact
- NPCs cannot share world knowledge contextually
- Lore discovery only works via DM manual grant
- Major feature (gradual lore revelation) not functional

### Fix Required
1. Add `discover_lore` to `tool_builder.rs`
2. Fetch NPC's known lore in `build_prompt()`
3. Add lore grant in tool executor

---

## Critical Gap #8: Observations Not Used in Dialogue Context

### Affected Systems
- Observation System
- Dialogue System

### The Problem

LLM context doesn't include which NPCs the PC has previously observed:
- `GamePromptRequest` has no field for known NPCs
- `StartConversation` doesn't fetch observation data

### Impact
- LLM can't reference "You've met Marcus before"
- NPCs can't acknowledge past encounters
- Knowledge-based dialogue impossible

### Fix Required
Add to `GamePromptRequest`:
```rust
pub known_npcs: Vec<KnownNpcContext>, // From observations
```

---

## Critical Gap #9: StagingReady Doesn't Record Observations

### Affected Systems
- Staging System
- Observation System

### The Problem

Only `EnterRegion` records observations. When DM approves staging with new NPCs for players already in region, those players don't get observations.

### Impact
- Player A in region, DM adds NPC to staging
- Player A sees NPC but no observation recorded
- Known NPCs panel stale

### Fix Required
Broadcast observation updates when staging changes NPCs for existing players.

---

## Priority Matrix

| Gap | Severity | Effort | Priority |
|-----|----------|--------|----------|
| #1 LLM Response Parsing | Critical | Medium | **P0** |
| #2 Tool Execution | Critical | Medium | **P0** |
| #3 Narrative Context | High | Low | **P1** |
| #4 CharacterStatUpdated | High | Low | **P1** |
| #5 Challenge Modifier | Medium | Low | **P1** |
| #6 Visual State | Medium | Medium | **P2** |
| #7 Lore Integration | Medium | High | **P2** |
| #8 Observation Context | Low | Low | **P2** |
| #9 Staging Observations | Low | Low | **P3** |

---

## Recommended Implementation Order

### Phase 1: Core Loop Fixes (P0)
1. Create LLM response parser for structured output
2. Implement tool executor for approved tools
3. Wire parser into approval flow

### Phase 2: Context & Feedback (P1)
4. Populate active narrative events in LLM context
5. Broadcast CharacterStatUpdated after stat changes
6. Fix challenge prompt to show correct modifier

### Phase 3: Visual & Knowledge (P2)
7. Send SceneChanged after StagingReady
8. Apply visual state in player UI
9. Add observation context to dialogue prompts

### Phase 4: Lore Integration (P2-P3)
10. Implement discover_lore tool
11. Include NPC lore knowledge in context
12. Broadcast observation updates on staging changes

---

## Related Documents

- [Dialogue System](../systems/dialogue-system.md)
- [Narrative System](../systems/narrative-system.md)
- [Challenge System](../systems/challenge-system.md)
- [Staging System](../systems/staging-system.md)
- [Lore System](../systems/lore-system.md)
- [Observation System](../systems/observation-system.md)

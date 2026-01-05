# Lore System

## Overview

The Lore System manages **world knowledge** that characters can discover, including historical events, legends, secrets, and common knowledge. Lore entries contain **chunks** that can be discovered individually or together, enabling gradual revelation of information through gameplay. Characters (both PCs and NPCs) track which lore they know via graph relationships.

---

## Game Design

The Lore System enriches worldbuilding and enables knowledge-based gameplay:

1. **Gradual Discovery**: Players piece together lore chunk by chunk through exploration, conversation, and investigation
2. **Character Knowledge**: Each character (PC or NPC) has their own knowledge graph - NPCs can share what they know
3. **Common Knowledge**: Some lore is universally known, automatically available to all characters
4. **Location-Based Knowledge**: Locations can have associated common lore that visitors learn
5. **LLM Integration**: The LLM can grant lore discovery during conversations based on context
6. **DM Control**: DM can manually grant or revoke lore knowledge at any time

### Knowledge Categories

| Category | Description | Example |
|----------|-------------|---------|
| Historical | Past events | "The Fall of House Valeren" |
| Legend | Myths, folklore | "The Dragon of Mistpeak" |
| Secret | Hidden truths | "The True Identity of the King" |
| Common | Widely known | "The Kingdom's Major Cities" |
| Technical | How things work | "How Arcane Crystals Function" |
| Political | Factions, alliances | "The Three Great Houses" |
| Natural | Geography, flora/fauna | "The Creatures of the Darkwood" |
| Religious | Beliefs, prophecies | "The Prophecy of the Chosen" |

---

## User Stories

### Pending

- [ ] **US-LORE-001**: As a DM, I can create lore entries with multiple chunks so that players discover information gradually
  - *Implementation*: Lore CRUD in Creator mode

- [ ] **US-LORE-002**: As a DM, I can link lore to characters, locations, regions, or items so that lore is contextually organized
  - *Implementation*: Neo4j ABOUT_* edges

- [ ] **US-LORE-003**: As a DM, I can mark lore as common knowledge so that all characters automatically know it
  - *Implementation*: `is_common_knowledge` flag on Lore entity

- [ ] **US-LORE-004**: As a DM, I can associate common lore with a location so that visitors learn it
  - *Implementation*: COMMON_LORE edge from Location to Lore

- [ ] **US-LORE-005**: As a player, I can view my character's known lore in a journal/codex
  - *Implementation*: Lore journal UI component

- [ ] **US-LORE-006**: As a player, I see partial lore entries when I only know some chunks
  - *Implementation*: Chunk-aware rendering with "unknown" placeholders

- [ ] **US-LORE-007**: As a DM, I can manually grant lore (full or specific chunks) to characters
  - *Implementation*: DM action in character panel

- [ ] **US-LORE-008**: As a DM, I can revoke lore knowledge from characters
  - *Implementation*: DM action to remove KNOWS_LORE edge

- [ ] **US-LORE-009**: As a DM, I can see which characters know which lore in an overview
  - *Implementation*: Knowledge matrix view

- [ ] **US-LORE-010**: The LLM can grant lore discovery during NPC conversations
  - *Implementation*: LLM tool for lore discovery with DM approval

---

## UI Mockups

### Lore Editor (Creator Mode)

```
+-----------------------------------------------------------------------------+
|  Lore Editor                                                       [X]       |
+-----------------------------------------------------------------------------+
|                                                                              |
|  Title: [The Fall of House Valeren                                    ]     |
|                                                                              |
|  Category: [v Historical    ]      [x] Common Knowledge                     |
|                                                                              |
|  Summary (DM reference):                                                     |
|  +------------------------------------------------------------------------+ |
|  | The destruction of House Valeren 200 years ago, and the curse that    | |
|  | followed. Key to understanding the current political tensions.         | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
|  Tags: [politics] [history] [valeren] [+]                                    |
|                                                                              |
|  --- Chunks (discoverable pieces) ------------------------------------      |
|                                                                              |
|  +------------------------------------------------------------------------+ |
|  | Chunk 1: "The Glory Days"                                    [^][v][x] | |
|  | +------------------------------------------------------------------+ | |
|  | | House Valeren was once the most powerful noble family in the    | | |
|  | | realm, controlling the eastern provinces for three centuries.   | | |
|  | +------------------------------------------------------------------+ | |
|  | Discovery hint: [Common knowledge in eastern provinces           ]   | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
|  +------------------------------------------------------------------------+ |
|  | Chunk 2: "The Betrayal"                                      [^][v][x] | |
|  | +------------------------------------------------------------------+ | |
|  | | Lord Valeren was betrayed by his own brother, who opened the    | | |
|  | | castle gates to enemy forces during the Siege of Ashford.       | | |
|  | +------------------------------------------------------------------+ | |
|  | Discovery hint: [Found in historical texts at the Grand Library  ]   | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
|  +------------------------------------------------------------------------+ |
|  | Chunk 3: "The Curse" (SECRET)                                [^][v][x] | |
|  | +------------------------------------------------------------------+ | |
|  | | With his dying breath, Lord Valeren cursed his brother's line.  | | |
|  | | The curse still affects descendants of the betrayer.            | | |
|  | +------------------------------------------------------------------+ | |
|  | Discovery hint: [Only known to Valeren descendants or scholars   ]   | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
|  [+ Add Chunk]                                                               |
|                                                                              |
|  --- Connections ----------------------------------------------------------  |
|                                                                              |
|  About Characters: [+ Add]                                                   |
|    - Lord Valeren (Historical)                                               |
|    - Baron Ashford (current - descendant of betrayer)                        |
|                                                                              |
|  About Locations: [+ Add]                                                    |
|    - Valeren Castle Ruins                                                    |
|    - Ashford Manor                                                           |
|                                                                              |
|  Common in Locations: [+ Add]                                                |
|    - Eastern Province (Chunk 1 only)                                         |
|                                                                              |
|  +--------------------+                                                      |
|  |   Save Lore       |                                                      |
|  +--------------------+                                                      |
|                                                                              |
+-----------------------------------------------------------------------------+
```

**Status**: Pending

### Player Lore Journal

```
+-----------------------------------------------------------------------------+
|  Codex                                                                       |
+-----------------------------------------------------------------------------+
|                                                                              |
|  [All] [Historical] [Legend] [Secret] [Political] [...]    [Search...]      |
|                                                                              |
|  --- Known Lore ----------------------------------------------------------  |
|                                                                              |
|  +------------------------------------------------------------------------+ |
|  | The Fall of House Valeren                              [Historical]    | |
|  | You know 2 of 3 parts                                                  | |
|  |                                                                        | |
|  | > The Glory Days                                                       | |
|  |   House Valeren was once the most powerful noble family...            | |
|  |                                                                        | |
|  | > The Betrayal                                                         | |
|  |   Lord Valeren was betrayed by his own brother...                     | |
|  |                                                                        | |
|  | > [Unknown - there may be more to discover]                           | |
|  |                                                                        | |
|  | Discovered: Day 3 (conversation with Old Historian)                   | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
|  +------------------------------------------------------------------------+ |
|  | The Three Great Houses                                     [Political] | |
|  | Complete                                                               | |
|  |                                                                        | |
|  | The kingdom is ruled by three great noble houses: Ashford of the     | |
|  | East, Thornwood of the West, and Stormhaven of the North...          | |
|  |                                                                        | |
|  | Discovered: Common knowledge                                          | |
|  +------------------------------------------------------------------------+ |
|                                                                              |
+-----------------------------------------------------------------------------+
```

**Status**: Pending

### DM Lore Grant Modal

```
+-----------------------------------------------------------------------------+
|  Grant Lore Knowledge                                              [X]       |
+-----------------------------------------------------------------------------+
|                                                                              |
|  Character: [v Aldric the Ranger        ]                                   |
|                                                                              |
|  Select Lore: [v The Fall of House Valeren   ]                              |
|                                                                              |
|  --- Chunks to Grant -----------------------------------------------------  |
|                                                                              |
|  [x] Chunk 1: "The Glory Days"         (character already knows)            |
|  [x] Chunk 2: "The Betrayal"           (character already knows)            |
|  [ ] Chunk 3: "The Curse"              (NEW)                                |
|                                                                              |
|  Source: [v DM Granted        ]                                             |
|          [  Conversation     ]                                              |
|          [  Investigation    ]                                              |
|          [  Read Book        ]                                              |
|                                                                              |
|  Notes (optional):                                                           |
|  [Discovered the curse while exploring Valeren ruins                    ]   |
|                                                                              |
|  +--------------------+                                                      |
|  |   Grant Knowledge  |                                                      |
|  +--------------------+                                                      |
|                                                                              |
+-----------------------------------------------------------------------------+
```

**Status**: Pending

---

## Data Model

### Neo4j Nodes

```cypher
// Lore - a piece of world knowledge
(:Lore {
    id: "uuid",
    world_id: "uuid",
    title: "The Fall of House Valeren",
    summary: "DM reference summary...",
    category: "historical",  // historical, legend, secret, common, technical, political, natural, religious
    chunks: [  // JSON array of chunks
        {
            id: "uuid",
            order: 0,
            title: "The Glory Days",
            content: "House Valeren was once...",
            discovery_hint: "Common knowledge in eastern provinces"
        },
        {
            id: "uuid",
            order: 1,
            title: "The Betrayal",
            content: "Lord Valeren was betrayed...",
            discovery_hint: "Found in historical texts"
        }
    ],
    is_common_knowledge: false,
    tags: ["politics", "history", "valeren"],
    created_at: datetime,
    updated_at: datetime
})
```

### Neo4j Edges

```cypher
// Lore is about a character
(lore:Lore)-[:ABOUT_CHARACTER]->(character:Character)

// Lore is about a location
(lore:Lore)-[:ABOUT_LOCATION]->(location:Location)

// Lore is about a region
(lore:Lore)-[:ABOUT_REGION]->(region:Region)

// Lore is about an item
(lore:Lore)-[:ABOUT_ITEM]->(item:Item)

// Character knows lore (with discovery details)
(character:Character)-[:KNOWS_LORE {
    known_chunk_ids: ["uuid1", "uuid2"],  // Empty = knows all
    discovery_source: "conversation",     // conversation, investigation, dm_granted, etc.
    source_details: "{npc_id: '...', npc_name: 'Old Historian'}",  // JSON
    discovered_at: datetime,              // Game time
    notes: "Learned while asking about local history"
}]->(lore:Lore)

// Location has common lore (visitors learn it)
(location:Location)-[:COMMON_LORE {
    chunk_ids: ["uuid1"]  // Optional: only specific chunks. Empty = all
}]->(lore:Lore)
```

---

## API

### REST Endpoints

| Method | Path | Description | Status |
|--------|------|-------------|--------|
| GET | `/api/worlds/{id}/lore` | List all lore in world | Pending |
| POST | `/api/worlds/{id}/lore` | Create lore entry | Pending |
| GET | `/api/lore/{id}` | Get lore by ID | Pending |
| PUT | `/api/lore/{id}` | Update lore | Pending |
| DELETE | `/api/lore/{id}` | Delete lore | Pending |
| GET | `/api/characters/{id}/lore` | Get character's known lore | Pending |
| POST | `/api/characters/{id}/lore` | Grant lore to character | Pending |
| DELETE | `/api/characters/{id}/lore/{lore_id}` | Revoke lore from character | Pending |

### WebSocket Messages

#### Client -> Server

| Message | Fields | Purpose |
|---------|--------|---------|
| `GrantLore` | `character_id`, `lore_id`, `chunk_ids`, `source`, `notes` | DM grants lore |
| `RevokeLore` | `character_id`, `lore_id` | DM revokes lore |

#### Server -> Client

| Message | Fields | Purpose |
|---------|--------|---------|
| `LoreDiscovered` | `character_id`, `lore`, `chunks`, `source` | Character discovered lore |
| `LoreRevoked` | `character_id`, `lore_id` | Character lost lore knowledge |
| `LoreUpdated` | `lore` | Lore entry was modified |

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| Lore Entity | Pending | - | `entities/lore.rs` |
| LoreChunk Value Object | Pending | - | Part of Lore entity |
| LoreKnowledge Edge | Pending | - | KNOWS_LORE edge data |
| LoreRepository | Pending | - | Neo4j CRUD |
| LoreService | Pending | - | Business logic |
| Protocol Messages | Pending | Pending | Lore-specific messages |
| WebSocket Handlers | Pending | Pending | Grant/revoke handlers |
| Lore Editor UI | - | Pending | Creator mode |
| Lore Journal UI | - | Pending | Player codex |
| DM Grant Modal | - | Pending | Manual grant UI |
| LLM Tool Integration | Pending | - | discover_lore tool |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `crates/domain/src/entities/lore.rs` | Lore, LoreChunk, LoreKnowledge |
| Domain | `crates/domain/src/ids.rs` | LoreId, LoreChunkId |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| UI | `crates/player-ui/src/presentation/components/creator/lore_editor.rs` | Lore editor (pending) |
| UI | `crates/player-ui/src/presentation/components/lore_journal.rs` | Player codex (pending) |

---

## Related Systems

- **Depends on**: [Character System](./character-system.md) (knowledge holders), [Navigation System](./navigation-system.md) (location-based common lore)
- **Used by**: [Dialogue System](./dialogue-system.md) (LLM can reference/grant lore), [Observation System](./observation-system.md) (similar knowledge tracking pattern)

---

## LLM Integration

The LLM has access to a `discover_lore` tool during conversations:

```json
{
  "name": "discover_lore",
  "description": "Grant lore knowledge to a character based on the conversation",
  "parameters": {
    "character_id": "uuid of character learning the lore",
    "lore_id": "uuid of lore being discovered",
    "chunk_ids": ["optional array of specific chunk IDs, empty for all"],
    "reasoning": "Why this character would learn this now"
  }
}
```

The DM sees proposed lore discoveries in the approval queue and can accept, modify, or reject them.

---

## Revision History

| Date | Change |
|------|--------|
| 2026-01-05 | Initial version - Phase 1 domain design |

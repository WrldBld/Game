# ADR-003: Neo4j as Primary Storage

## Status

Accepted

## Date

2026-01-13

## Context

WrldBldr is a tabletop RPG engine that needs to store:
- Entities with complex relationships (characters, locations, items, scenes)
- Rich interconnections (character relationships, location connections, narrative events)
- Dynamic queries for LLM context building (traversing relationships to build prompts)
- Multiplayer state (player positions, inventory, observations)

The data model is inherently graph-like:
- Characters have relationships with other characters
- Locations connect to other locations via exits
- NPCs have observations about players
- Narrative events trigger based on traversable conditions

## Decision

Use **Neo4j** as the primary database with a graph-first data model:

1. **All entities as nodes** with typed IDs
2. **Relationships as edges** with properties (timestamps, reasons, strengths)
3. **JSON only for non-relational data** (configuration blobs, templates)
4. **Query-first schema design** optimized for common traversals

## Consequences

### Positive

- Natural fit for relationship-heavy data model
- Flexible queries for LLM context building (e.g., "find all characters within 2 hops who have negative relationships")
- Easy to add new relationship types without schema migrations
- Cypher query language is intuitive for graph traversals
- Built-in support for path finding and graph algorithms

### Negative

- Less mature ecosystem than PostgreSQL
- Hosting options more limited (self-host or Aura)
- Team needs to learn Cypher
- Less tooling for ORMs and migrations
- Can be slower for simple key-value lookups than Redis

### Neutral

- Requires thinking in graphs rather than tables
- Schema is more flexible but less enforced

## Alternatives Considered

### 1. PostgreSQL with Foreign Keys

Traditional relational model with join tables for relationships.

**Rejected:** Would require complex JOINs for relationship traversals. Queries like "find all NPCs who have heard about this character from another NPC" become unwieldy.

### 2. PostgreSQL with JSON Fields

Store relationships in JSON columns.

**Rejected:** Loses query ability on relationships. Can't efficiently query "all characters with relationship strength > 50".

### 3. MongoDB

Document store with embedded relationships.

**Rejected:** Embedding creates duplication, references create N+1 queries. Neither approach handles the graph-like queries we need.

### 4. Mixed Storage (Postgres + Redis)

Relational for entities, Redis for relationships and caching.

**Rejected:** Adds operational complexity. Would need to keep two stores in sync.

## Implementation Notes

- Use typed IDs (`CharacterId`, not raw `Uuid`) at application layer
- Always use Cypher parameters (never string concatenation)
- Indexes on frequently queried properties
- Relationship properties for metadata (created_at, strength, reason)

## References

- [neo4j-schema.md](neo4j-schema.md) - Full schema documentation
- Neo4j documentation: https://neo4j.com/docs/

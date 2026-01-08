# Implementation Gaps Plan

This plan tracks gaps between the systems docs, protocol, and current engine/player wiring.
Update this file as work progresses.

## Status Legend
- [ ] Pending
- [~] In progress
- [x] Done

- [x] Implement request handlers for PlayerCharacter/Relationship/Observation (Priority A)
- [x] Implement request handlers for World/Character/Location/Region CRUD (Priority A)
- [x] Implement request handlers for Challenge/NarrativeEvent/EventChain (Priority B)
- [x] Implement request handlers for Goal/Want/Actantial (Priority C)
- [ ] Implement request handlers for Skill/Act/Interaction/Scene (Priority D)
- [x] Implement missing StoryEvent operations (Get/Update) (Priority E)
- [x] Implement missing NPC requests (disposition/relationship) (Priority F)
- [ ] Implement missing AI requests (SuggestDeflectionBehavior, SuggestBehavioralTells) (Priority G)

## Phase 2: HTTP API Coverage (Engine)
- [ ] Add settings endpoints: /api/settings, /api/settings/metadata, /api/settings/reset
- [ ] Add world settings endpoints: /api/worlds/{id}/settings, /api/worlds/{id}/settings/reset
- [ ] Add rule system presets endpoint used by player

## Phase 3: Use Case Wiring and UX Validation
- [ ] Wire world import use case to API entry point (HTTP or WebSocket)
- [ ] Wire queue use cases to API entry points or background runner
- [ ] Extract join/login logic from WebSocket handlers into use cases
- [ ] Audit time flow (suggestions -> approval -> advance) against UI

## Phase 4: Documentation Reconciliation
- [x] Update systems docs to match implemented handlers and UI paths
- [~] Update MVP and ROADMAP to reflect actual implementation state
- [x] Note remaining gaps and planned sequencing in ROADMAP

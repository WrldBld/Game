# Encapsulation Remediation Plan

## Overview

This plan addresses issues identified in the comprehensive code review focusing on:
1. **ADR-009 Compliance** - Complete repository layer elimination
2. **ADR-008 Tier 4 Compliance** - Convert over-encapsulated entities to public fields
3. **Value Object Consistency** - Align UserId with other newtypes
4. **Optional Improvements** - Minor enhancements to existing patterns

**Estimated Total Effort:** 8-12 hours

**Validation Status:**
- All findings confirmed through code review with sub-agent exploration
- Files and line counts verified against current codebase state

---

## Priority 1: CRITICAL - Complete ADR-009 Repository Layer Elimination

**Issue**: Despite ADR-009 mandating elimination of the repository wrapper layer, 2 repository wrappers remain and 3 use cases still import them.

**Current State (Non-Compliant):**
- `crates/engine/src/repositories/` still exists with 189 lines
- `AssetsRepository` wraps `Arc<dyn AssetRepo>` + `Arc<dyn ImageGenPort>`
- `SettingsRepository` wraps `Arc<dyn SettingsRepo>` with caching logic
- 3 use cases import repository wrappers instead of port traits

### Phase 1.1: Eliminate AssetsRepository

**Files to Modify:**

#### 1.1.1 `crates/engine/src/use_cases/assets/mod.rs`

**Current (Non-Compliant):**
```rust
// Line ~19
use crate::repositories::AssetsRepository;

// Line ~60
pub struct GenerateAsset {
    assets: Arc<AssetsRepository>,
    queue: Arc<QueueService>,
    clock: Arc<ClockService>,
}
```

**After (Compliant):**
```rust
use crate::infrastructure::ports::{AssetRepo, ImageGenPort, QueuePort, ClockPort};

pub struct GenerateAsset {
    asset_repo: Arc<dyn AssetRepo>,
    image_gen: Arc<dyn ImageGenPort>,
    queue: Arc<dyn QueuePort>,
    clock: Arc<dyn ClockPort>,
}

impl GenerateAsset {
    pub fn new(
        asset_repo: Arc<dyn AssetRepo>,
        image_gen: Arc<dyn ImageGenPort>,
        queue: Arc<dyn QueuePort>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self { asset_repo, image_gen, queue, clock }
    }

    // Update execute() to use self.asset_repo and self.image_gen directly
}
```

**Changes Required:**
- Replace `Arc<AssetsRepository>` with `Arc<dyn AssetRepo>` + `Arc<dyn ImageGenPort>`
- Update constructor to accept both ports
- Update all method calls from `self.assets.repo.*` to `self.asset_repo.*`
- Update all method calls from `self.assets.image_gen.*` to `self.image_gen.*`

#### 1.1.2 `crates/engine/src/use_cases/assets/expression_sheet.rs`

**Current (Non-Compliant):**
```rust
// Line ~31
use crate::repositories::AssetsRepository;

// Line ~136
pub struct GenerateExpressionSheet {
    assets: Arc<AssetsRepository>,
    character: Arc<CharacterRepository>,
    queue: Arc<QueueService>,
    clock: Arc<ClockService>,
}
```

**After (Compliant):**
```rust
use crate::infrastructure::ports::{AssetRepo, ImageGenPort, CharacterRepo, QueuePort, ClockPort};

pub struct GenerateExpressionSheet {
    asset_repo: Arc<dyn AssetRepo>,
    image_gen: Arc<dyn ImageGenPort>,
    character: Arc<dyn CharacterRepo>,
    queue: Arc<dyn QueuePort>,
    clock: Arc<dyn ClockPort>,
}
```

**Changes Required:**
- Same pattern as mod.rs
- Note: `CharacterRepository` is already defined as `type CharacterRepository = dyn CharacterRepo;` on line 34, which is acceptable

### Phase 1.2: Eliminate SettingsRepository

**File:** `crates/engine/src/repositories/settings.rs` (102 lines)

**Analysis:** Contains caching logic for world settings fallback to global settings.

**Decision:** Move the fallback logic to a use case or inline in handlers.

**Files to Modify:**

#### 1.2.1 Create or update settings use case

**Option A (Recommended):** Inline the fallback logic in the existing settings management use case.

**Current SettingsRepository logic to preserve:**
```rust
pub async fn get_for_world(&self, world_id: WorldId) -> Result<AppSettings, SettingsError> {
    if let Some(settings) = self.repo.get_for_world(world_id).await? {
        let settings = settings.with_world_id(Some(world_id));
        return Ok(settings);
    }

    // Fallback to global settings
    let global = self.get_global().await?;
    Ok(AppSettings::for_world(global, world_id))
}
```

**After (in use case or handler):**
```rust
// Direct port injection
let settings: Arc<dyn SettingsRepo> = ...;

// Logic inlined
let world_settings = settings.get_for_world(world_id).await?;
let result = match world_settings {
    Some(s) => s.with_world_id(Some(world_id)),
    None => {
        let global = settings.get_global().await?
            .ok_or(SettingsError::NotConfigured)?;
        AppSettings::for_world(global, world_id)
    }
};
```

### Phase 1.3: Delete Repository Files

**Files to Delete:**
```
crates/engine/src/repositories/assets.rs    (67 lines)
crates/engine/src/repositories/settings.rs  (102 lines)
crates/engine/src/repositories/mod.rs       (20 lines)
```

**Total:** 189 lines removed

### Phase 1.4: Update App Composition

**File:** `crates/engine/src/app.rs`

**Current:**
```rust
let assets_repo = Arc::new(AssetsRepository::new(
    repos.asset.clone(),
    image_gen.clone(),
));
```

**After:**
```rust
// Pass ports directly to use cases
GenerateAsset::new(
    repos.asset.clone(),
    image_gen.clone(),
    queue.clone(),
    clock.clone(),
)
```

### Verification

```bash
# Ensure repositories directory is deleted
ls crates/engine/src/repositories/ 2>/dev/null && echo "FAIL" || echo "PASS"

# Ensure no repository imports remain in use cases
rg "use crate::repositories::" crates/engine/src/use_cases/ --type rust
# Should return nothing

# Build and test
cargo build -p wrldbldr-engine
cargo test -p wrldbldr-engine --lib
```

### Status: TODO

**Effort Estimate:** 2-3 hours

---

## Priority 2: HIGH - Entity Over-Encapsulation Remediation

**Issue**: Multiple entity files in `crates/domain/src/entities/` have private fields + trivial accessors for data that has no invariants to protect. Per ADR-008 Tier 4, simple data structs should use public fields.

**Scope:** 5 files, ~400 accessor/builder methods to remove, ~50 fields to make public

### Phase 2.1: Convert skill.rs (Simplest)

**File:** `crates/domain/src/entities/skill.rs` (700 lines)

**Current Pattern (Over-Encapsulated):**
```rust
pub struct Skill {
    id: SkillId,
    world_id: WorldId,
    name: String,
    description: String,
    category: SkillCategory,
    base_attribute: Option<String>,
    is_custom: bool,
    is_hidden: bool,
    order: u32,
}

impl Skill {
    pub fn id(&self) -> SkillId { self.id }
    pub fn world_id(&self) -> WorldId { self.world_id }
    pub fn name(&self) -> &str { &self.name }
    pub fn description(&self) -> &str { &self.description }
    pub fn category(&self) -> SkillCategory { self.category }
    pub fn base_attribute(&self) -> Option<&str> { self.base_attribute.as_deref() }
    pub fn is_custom(&self) -> bool { self.is_custom }
    pub fn is_hidden(&self) -> bool { self.is_hidden }
    pub fn order(&self) -> u32 { self.order }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
    // ... 8 more builder methods
}
```

**After (ADR-008 Tier 4 Compliant):**
```rust
/// A skill definition in the game system.
///
/// ADR-008 Tier 4: Simple data struct with no invariants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: SkillId,
    pub world_id: WorldId,
    pub name: String,
    pub description: String,
    pub category: SkillCategory,
    pub base_attribute: Option<String>,
    pub is_custom: bool,
    pub is_hidden: bool,
    pub order: u32,
}

impl Skill {
    /// Create a new skill with required fields.
    pub fn new(world_id: WorldId, name: impl Into<String>, category: SkillCategory) -> Self {
        Self {
            id: SkillId::new(),
            world_id,
            name: name.into(),
            description: String::new(),
            category,
            base_attribute: None,
            is_custom: false,
            is_hidden: false,
            order: 0,
        }
    }
}
```

**Changes:**
- Add `pub` to all 9 fields
- Remove 9 trivial accessor methods (~27 lines)
- Remove 9 builder methods (~36 lines)
- Keep `::new()` constructor for convenience
- Add ADR-008 comment documenting design decision

**Impact:**
- Update all call sites from `skill.name()` to `skill.name`
- Update all call sites from `skill.with_description(...)` to `skill.description = ...`

### Phase 2.2: Convert feat.rs

**File:** `crates/domain/src/entities/feat.rs` (533 lines)

**Fields to make public:** 8 (id, system_id, name, description, prerequisites, benefits, source, category, repeatable, tags)

**Methods to remove:**
- 10 trivial accessors
- 7 builder methods

**Pattern:** Same as skill.rs

**Note:** `AbilityUses` struct already has public fields - no changes needed there.

### Phase 2.3: Convert spell.rs (Spell struct only)

**File:** `crates/domain/src/entities/spell.rs` (677 lines)

**Fields to make public on Spell:** 16 (id, system_id, name, level, school, casting_time, range, components, duration, description, higher_levels, classes, source, tags, ritual, concentration)

**Methods to remove:**
- 15 trivial accessors
- 8 builder methods

**Note:** Sub-types already have public fields:
- `CastingTime` - 3 public fields (no change)
- `SpellComponents` - 3 public fields (no change)
- `MaterialComponent` - 3 public fields (no change)

### Phase 2.4: Convert item.rs

**File:** `crates/domain/src/entities/item.rs` (231 lines)

**Fields to make public:** 9 (id, world_id, name, description, item_type, is_unique, properties, can_contain_items, container_limit)

**Methods to remove:**
- 9 trivial accessors
- 8 builder methods

**Special Note:** There's coupling between `can_contain_items` and `container_limit`. Add documentation:

```rust
/// An item in the game world.
///
/// ADR-008 Tier 4: Simple data struct.
///
/// Note: When setting `container_limit`, also set `can_contain_items = true`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub world_id: WorldId,
    pub name: String,
    // ...
    /// Whether this item can contain other items.
    /// Set to true when `container_limit` is set.
    pub can_contain_items: bool,
    /// Maximum items this container can hold (if can_contain_items is true).
    pub container_limit: Option<u32>,
}
```

### Phase 2.5: Convert class_feature.rs (3 structs)

**File:** `crates/domain/src/entities/class_feature.rs` (519 lines)

**Structs to convert:**

1. **ClassFeature** - 10 fields → public
2. **RacialTrait** - 9 fields → public
3. **BackgroundFeature** - 6 fields → public

**Total methods to remove:** ~40 (accessors + builders)

**Note:** `FeatureUses` struct already has public fields - no changes needed.

### Phase 2.6: DO NOT Convert character_content.rs

**File:** `crates/domain/src/entities/character_content.rs` (772 lines)

**Status:** KEEP ENCAPSULATED

**Reason:** This file contains real domain logic and invariants:
- `CharacterSpells.use_slot()` - validates slot availability
- `SpellSlotPool` - enforces `current <= max`
- `CharacterIdentity.add_class()` - maintains total_level sum
- `ActiveFeature.use_once()` - enforces use constraints

These are NOT simple data structs - they are aggregate-like types protecting game rules.

### Update Call Sites

After converting to public fields, all call sites need updating:

**Search patterns to find:**
```bash
# Find accessor calls to update
rg '\.name\(\)' crates/ --type rust -l
rg '\.description\(\)' crates/ --type rust -l
rg '\.category\(\)' crates/ --type rust -l

# Find builder calls to update
rg '\.with_name\(' crates/ --type rust -l
rg '\.with_description\(' crates/ --type rust -l
```

**Conversion pattern:**
```rust
// BEFORE
let name = skill.name();
let skill = Skill::new(...).with_description("...");

// AFTER
let name = &skill.name;
let mut skill = Skill::new(...);
skill.description = "...".to_string();
```

### Verification

```bash
# Count accessor methods (should decrease significantly)
rg 'pub fn \w+\(&self\) ->' crates/domain/src/entities/ --type rust | wc -l
# Before: ~360, After: ~100 (only character_content.rs methods remain)

# Count builder methods (should decrease significantly)
rg 'pub fn with_\w+\(mut self' crates/domain/src/entities/ --type rust | wc -l
# Before: ~187, After: ~50

# Build and test
cargo build --workspace
cargo test --workspace
```

### Status: TODO

**Effort Estimate:** 4-6 hours (including call site updates)

---

## Priority 3: MEDIUM - UserId Consistency

**Issue**: `UserId::new()` returns `Result<Self, &'static str>` while all other newtypes return `Result<Self, DomainError>`.

**File:** `crates/domain/src/ids.rs` (lines 96-104)

**Current:**
```rust
impl UserId {
    pub fn new(id: impl Into<String>) -> Result<Self, &'static str> {
        let id = id.into();
        if id.is_empty() {
            return Err("UserId cannot be empty");
        }
        Ok(Self(id))
    }
}
```

**After:**
```rust
use crate::error::DomainError;

impl UserId {
    pub fn new(id: impl Into<String>) -> Result<Self, DomainError> {
        let id = id.into();
        if id.is_empty() {
            return Err(DomainError::validation("UserId cannot be empty"));
        }
        Ok(Self(id))
    }
}
```

**Also add serde integration:**
```rust
impl TryFrom<String> for UserId {
    type Error = DomainError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<UserId> for String {
    fn from(id: UserId) -> String {
        id.0
    }
}
```

**Update call sites:**
```bash
rg 'UserId::new\(' crates/ --type rust -l
# Update any .map_err() calls that expect &'static str
```

### Verification

```bash
cargo build --workspace
cargo test --workspace --lib
```

### Status: TODO

**Effort Estimate:** 30-45 minutes

---

## Priority 4: LOW - Optional Improvements

### 4.1 Add Serde Validation to DiceFormula

**File:** `crates/domain/src/value_objects/dice.rs`

**Current:** No `#[serde(try_from)]` attribute

**After:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DiceFormula {
    dice_count: u8,
    die_size: u8,
    modifier: i32,
}

impl TryFrom<String> for DiceFormula {
    type Error = DiceParseError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s)
    }
}

impl From<DiceFormula> for String {
    fn from(formula: DiceFormula) -> String {
        formula.to_string()
    }
}
```

### 4.2 Document Entity Encapsulation Decisions

Add ADR-008 tier comments to all entity files:

```rust
/// A skill definition in the game system.
///
/// # Encapsulation (ADR-008)
///
/// This is a **Tier 4 Simple Data Struct** with public fields because:
/// - No invariants to protect
/// - No validation required between fields
/// - Just groups related data for persistence and transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill { ... }
```

### 4.3 Add validate() Method to Complex Value Objects

For types like `ActivationRules` and `AdHocOutcomes` that have complex structures but no constructor validation:

```rust
impl ActivationRules {
    /// Validate the activation rules configuration.
    pub fn validate(&self) -> Result<(), DomainError> {
        // Check for conflicting rules
        // Validate referenced IDs exist
        // etc.
    }
}
```

### Status: TODO

**Effort Estimate:** 1-2 hours

---

## Execution Order

1. **Priority 1** (ADR-009) - Complete repository elimination first
   - Blocks nothing, improves architecture clarity
   - 2-3 hours

2. **Priority 2** (Entity conversion) - Convert simple entities to public fields
   - May have broad call site impact
   - 4-6 hours

3. **Priority 3** (UserId) - Quick consistency fix
   - Low risk, isolated change
   - 30-45 minutes

4. **Priority 4** (Optional) - Nice-to-have improvements
   - Do if time permits
   - 1-2 hours

---

## Summary of Changes

### Files to Delete (189 lines)

| File | Lines | Reason |
|------|-------|--------|
| `crates/engine/src/repositories/assets.rs` | 67 | ADR-009 - replace with direct port injection |
| `crates/engine/src/repositories/settings.rs` | 102 | ADR-009 - move logic to use case |
| `crates/engine/src/repositories/mod.rs` | 20 | ADR-009 - directory eliminated |

### Files to Modify

| File | Change | Lines Removed | Lines Added |
|------|--------|---------------|-------------|
| `use_cases/assets/mod.rs` | Inject ports directly | ~10 | ~15 |
| `use_cases/assets/expression_sheet.rs` | Inject ports directly | ~10 | ~15 |
| `entities/skill.rs` | Public fields | ~63 | ~10 |
| `entities/feat.rs` | Public fields | ~51 | ~10 |
| `entities/spell.rs` | Public fields | ~69 | ~10 |
| `entities/item.rs` | Public fields | ~51 | ~10 |
| `entities/class_feature.rs` | Public fields (3 structs) | ~120 | ~20 |
| `domain/ids.rs` | UserId uses DomainError | ~5 | ~15 |

### Metrics

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Repository wrapper files | 3 | 0 | -100% |
| ADR-009 violations | 3 | 0 | -100% |
| Trivial accessor methods | ~360 | ~100 | -72% |
| Trivial builder methods | ~187 | ~50 | -73% |
| Lines of boilerplate | ~600 | ~150 | -75% |

---

## Final Verification Checklist

```bash
# 1. Repository layer eliminated
ls crates/engine/src/repositories/ 2>/dev/null && echo "FAIL: repos exist" || echo "PASS"

# 2. No repository imports in use cases
rg "use crate::repositories::" crates/engine/src/use_cases/ --type rust
# Should return nothing

# 3. Accessor count reduced
rg 'pub fn \w+\(&self\) ->' crates/domain/src/entities/ --type rust | wc -l
# Should be ~100 (down from ~360)

# 4. Build passes
cargo build --workspace

# 5. All tests pass
cargo test --workspace

# 6. Clippy clean
cargo clippy --workspace -- -D warnings

# 7. E2E tests pass
task e2e
```

---

## Progress Tracking

- [x] **Priority 1: ADR-009 Compliance**
  - [x] Phase 1.1: Eliminate AssetsRepository
  - [x] Phase 1.2: Eliminate SettingsRepository
  - [x] Phase 1.3: Delete repository files
  - [x] Phase 1.4: Update App composition

- [x] **Priority 2: Entity Over-Encapsulation**
  - [x] Phase 2.1: Convert skill.rs
  - [x] Phase 2.2: Convert feat.rs
  - [x] Phase 2.3: Convert spell.rs
  - [x] Phase 2.4: Convert item.rs
  - [x] Phase 2.5: Convert class_feature.rs
  - [x] Update all call sites

- [x] **Priority 3: UserId Consistency**
  - [x] Change return type to DomainError
  - [x] Add serde integration
  - [x] Update call sites

- [x] **Priority 4: Optional Improvements**
  - [x] Add serde validation to DiceFormula
  - [x] Add ADR-008 documentation comments
  - [x] Add validate() methods to complex VOs

---

## Related Documents

| Document | Purpose |
|----------|---------|
| [ADR-008](../architecture/ADR-008-tiered-encapsulation.md) | Tiered encapsulation decision |
| [ADR-009](../architecture/ADR-009-repository-layer-elimination.md) | Repository elimination decision |
| [AGENTS.md](../../AGENTS.md) | Architecture overview |
| [review.md](../architecture/review.md) | Code review guidelines |
| [code-review-plan.md](./code-review-plan.md) | Previous code review plan |

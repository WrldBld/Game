# Test Data

This directory contains JSON fixtures for integration testing and example worlds.

## Structure

```
test_data/
├── dnd5e/                    # D&D 5e example world and characters
│   ├── world.json            # Example world configuration
│   ├── characters/           # Pre-built character fixtures
│   │   ├── fighter_5.json    # Level 5 Fighter
│   │   ├── wizard_3.json     # Level 3 Wizard
│   │   └── multiclass.json   # Fighter 3 / Wizard 2
│   └── triggers/             # Narrative trigger fixtures
│       └── compendium.json   # Compendium-based triggers
└── README.md                 # This file
```

## Usage

### In Integration Tests

```rust
use crate::test_fixtures::load_fixture;

#[tokio::test]
async fn test_fighter_triggers() {
    let pc = load_fixture::<PlayerCharacter>("dnd5e/characters/fighter_5.json");
    // ... test logic
}
```

### Seeding Example Worlds

The fixtures can also be used to seed demonstration worlds via the API.

## Contributing

When adding new fixtures:
1. Use realistic D&D 5e values
2. Include comments in JSON where helpful
3. Test that fixtures deserialize correctly
4. Update this README with new fixture descriptions

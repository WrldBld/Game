//! Integration tests for narrative triggers with character data.
//!
//! These tests verify that compendium-based triggers (HasClass, HasOrigin,
//! KnowsSpell, HasFeat, etc.) correctly match against character sheet data.

use wrldbldr_domain::{
    NarrativeEvent, NarrativeEventName, NarrativeTrigger, NarrativeTriggerType, TriggerLogic,
    WorldId,
};

use crate::test_fixtures::{characters, trigger_context_from_pc, triggers};

// =============================================================================
// HasClass Trigger Tests
// =============================================================================

#[test]
fn test_has_class_trigger_fires_with_fighter() {
    // Setup: Load Fighter level 5 character
    let pc = characters::fighter_5();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with HasClass trigger for Fighter level 3+
    let trigger = triggers::has_class_fighter(Some(3));
    let event = create_test_event_with_trigger("fighter_test", trigger);

    // Assert: Trigger should match (Fighter 5 >= 3)
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        eval.is_triggered,
        "Fighter 5 should match HasClass(fighter, 3)"
    );
}

#[test]
fn test_has_class_trigger_fails_below_min_level() {
    // Setup: Load Fighter level 5 character
    let pc = characters::fighter_5();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with HasClass trigger for Fighter level 10+
    let trigger = triggers::has_class_fighter(Some(10));
    let event = create_test_event_with_trigger("fighter_high_level_test", trigger);

    // Assert: Trigger should NOT match (Fighter 5 < 10)
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        !eval.is_triggered,
        "Fighter 5 should NOT match HasClass(fighter, 10)"
    );
}

#[test]
fn test_has_class_trigger_without_min_level() {
    // Setup: Load Wizard level 3 character
    let pc = characters::wizard_3();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with HasClass trigger for Wizard (any level)
    let trigger = triggers::has_class_wizard(None);
    let event = create_test_event_with_trigger("wizard_any_level_test", trigger);

    // Assert: Trigger should match
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        eval.is_triggered,
        "Wizard 3 should match HasClass(wizard, None)"
    );
}

#[test]
fn test_has_class_trigger_multiclass_fighter() {
    // Setup: Load multiclass Fighter 3 / Wizard 2 character
    let pc = characters::multiclass();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with HasClass trigger for Fighter level 3+
    let trigger = triggers::has_class_fighter(Some(3));
    let event = create_test_event_with_trigger("multiclass_fighter_test", trigger);

    // Assert: Trigger should match (Fighter 3 >= 3)
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        eval.is_triggered,
        "Multiclass with Fighter 3 should match HasClass(fighter, 3)"
    );
}

#[test]
fn test_has_class_trigger_multiclass_wizard() {
    // Setup: Load multiclass Fighter 3 / Wizard 2 character
    let pc = characters::multiclass();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with HasClass trigger for Wizard level 3+ (character only has Wizard 2)
    let trigger = triggers::has_class_wizard(Some(3));
    let event = create_test_event_with_trigger("multiclass_wizard_test", trigger);

    // Assert: Trigger should NOT match (Wizard 2 < 3)
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        !eval.is_triggered,
        "Multiclass with Wizard 2 should NOT match HasClass(wizard, 3)"
    );
}

#[test]
fn test_has_class_trigger_fighter_does_not_match_wizard_character() {
    // Setup: Load Wizard level 3 character (no Fighter levels)
    let pc = characters::wizard_3();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with HasClass trigger for Fighter
    let trigger = triggers::has_class_fighter(None);
    let event = create_test_event_with_trigger("wizard_not_fighter_test", trigger);

    // Assert: Trigger should NOT match
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        !eval.is_triggered,
        "Wizard should NOT match HasClass(fighter)"
    );
}

// =============================================================================
// HasOrigin Trigger Tests
// =============================================================================

#[test]
fn test_has_origin_trigger_fires_with_elf() {
    // Setup: Load Wizard (Elf) character
    let pc = characters::wizard_3();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with HasOrigin trigger for Elf
    let trigger = triggers::has_origin("elf", "Elf");
    let event = create_test_event_with_trigger("elf_origin_test", trigger);

    // Assert: Trigger should match
    let eval = event.evaluate_triggers(&ctx);
    assert!(eval.is_triggered, "Elf wizard should match HasOrigin(elf)");
}

#[test]
fn test_has_origin_trigger_fires_with_human() {
    // Setup: Load Fighter (Human) character
    let pc = characters::fighter_5();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with HasOrigin trigger for Human
    let trigger = triggers::has_origin("human", "Human");
    let event = create_test_event_with_trigger("human_origin_test", trigger);

    // Assert: Trigger should match
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        eval.is_triggered,
        "Human fighter should match HasOrigin(human)"
    );
}

#[test]
fn test_has_origin_trigger_fails_with_wrong_race() {
    // Setup: Load Fighter (Human) character
    let pc = characters::fighter_5();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with HasOrigin trigger for Dwarf
    let trigger = triggers::has_origin("dwarf", "Dwarf");
    let event = create_test_event_with_trigger("human_not_dwarf_test", trigger);

    // Assert: Trigger should NOT match
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        !eval.is_triggered,
        "Human fighter should NOT match HasOrigin(dwarf)"
    );
}

// =============================================================================
// KnowsSpell Trigger Tests
// =============================================================================

#[test]
fn test_knows_spell_trigger_fires_with_fireball() {
    // Setup: Load Wizard character (knows Fireball)
    let pc = characters::wizard_3();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with KnowsSpell trigger for Fireball
    let trigger = triggers::knows_spell("fireball", "Fireball");
    let event = create_test_event_with_trigger("knows_fireball_test", trigger);

    // Assert: Trigger should match
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        eval.is_triggered,
        "Wizard should match KnowsSpell(fireball)"
    );
}

#[test]
fn test_knows_spell_trigger_fires_with_shield() {
    // Setup: Load Multiclass character (knows Shield)
    let pc = characters::multiclass();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with KnowsSpell trigger for Shield
    let trigger = triggers::knows_spell("shield", "Shield");
    let event = create_test_event_with_trigger("knows_shield_test", trigger);

    // Assert: Trigger should match
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        eval.is_triggered,
        "Multiclass should match KnowsSpell(shield)"
    );
}

#[test]
fn test_knows_spell_trigger_fails_with_unknown_spell() {
    // Setup: Load Wizard character
    let pc = characters::wizard_3();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with KnowsSpell trigger for Wish (not known)
    let trigger = triggers::knows_spell("wish", "Wish");
    let event = create_test_event_with_trigger("unknown_spell_test", trigger);

    // Assert: Trigger should NOT match
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        !eval.is_triggered,
        "Wizard should NOT match KnowsSpell(wish)"
    );
}

#[test]
fn test_knows_spell_trigger_fails_for_non_caster() {
    // Setup: Load Fighter character (no spells)
    let pc = characters::fighter_5();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with KnowsSpell trigger for any spell
    let trigger = triggers::knows_spell("magic_missile", "Magic Missile");
    let event = create_test_event_with_trigger("fighter_no_spells_test", trigger);

    // Assert: Trigger should NOT match
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        !eval.is_triggered,
        "Fighter should NOT match KnowsSpell(magic_missile)"
    );
}

// =============================================================================
// HasFeat Trigger Tests
// =============================================================================

#[test]
fn test_has_feat_trigger_fires_with_great_weapon_master() {
    // Setup: Load Fighter character (has Great Weapon Master)
    let pc = characters::fighter_5();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with HasFeat trigger for Great Weapon Master
    let trigger = triggers::has_feat("great_weapon_master", "Great Weapon Master");
    let event = create_test_event_with_trigger("gwm_test", trigger);

    // Assert: Trigger should match
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        eval.is_triggered,
        "Fighter should match HasFeat(great_weapon_master)"
    );
}

#[test]
fn test_has_feat_trigger_fires_with_war_caster() {
    // Setup: Load Multiclass character (has War Caster)
    let pc = characters::multiclass();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with HasFeat trigger for War Caster
    let trigger = triggers::has_feat("war_caster", "War Caster");
    let event = create_test_event_with_trigger("war_caster_test", trigger);

    // Assert: Trigger should match
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        eval.is_triggered,
        "Multiclass should match HasFeat(war_caster)"
    );
}

#[test]
fn test_has_feat_trigger_fails_without_feat() {
    // Setup: Load Wizard character (no feats)
    let pc = characters::wizard_3();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with HasFeat trigger for Sentinel
    let trigger = triggers::has_feat("sentinel", "Sentinel");
    let event = create_test_event_with_trigger("no_sentinel_test", trigger);

    // Assert: Trigger should NOT match
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        !eval.is_triggered,
        "Wizard should NOT match HasFeat(sentinel)"
    );
}

// =============================================================================
// Case Insensitivity Tests
// =============================================================================

#[test]
fn test_trigger_context_case_insensitive_class_matching() {
    // Setup: Load Fighter (stored as "fighter" lowercase)
    let pc = characters::fighter_5();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with HasClass trigger using uppercase
    let trigger = NarrativeTriggerType::HasClass {
        class_id: "FIGHTER".to_string(),
        class_name: "Fighter".to_string(),
        min_level: None,
    };
    let event = create_test_event_with_trigger("case_test_class", trigger);

    // Assert: Trigger should match (case insensitive)
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        eval.is_triggered,
        "HasClass matching should be case insensitive"
    );
}

#[test]
fn test_trigger_context_case_insensitive_origin_matching() {
    // Setup: Load Elf wizard (stored as "elf" lowercase)
    let pc = characters::wizard_3();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with HasOrigin trigger using uppercase
    let trigger = NarrativeTriggerType::HasOrigin {
        origin_id: "ELF".to_string(),
        origin_name: "Elf".to_string(),
    };
    let event = create_test_event_with_trigger("case_test_origin", trigger);

    // Assert: Trigger should match (case insensitive)
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        eval.is_triggered,
        "HasOrigin matching should be case insensitive"
    );
}

#[test]
fn test_trigger_context_case_insensitive_spell_matching() {
    // Setup: Load Wizard with "fireball" spell
    let pc = characters::wizard_3();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with KnowsSpell trigger using mixed case
    let trigger = NarrativeTriggerType::KnowsSpell {
        spell_id: "FireBall".to_string(),
        spell_name: "Fireball".to_string(),
    };
    let event = create_test_event_with_trigger("case_test_spell", trigger);

    // Assert: Trigger should match (case insensitive)
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        eval.is_triggered,
        "KnowsSpell matching should be case insensitive"
    );
}

// =============================================================================
// Combined Triggers Tests
// =============================================================================

#[test]
fn test_multiple_triggers_all_logic() {
    // Setup: Load Wizard character
    let pc = characters::wizard_3();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with multiple triggers (ALL logic)
    let event = create_test_event_with_triggers(
        "combined_all_test",
        vec![
            triggers::has_class_wizard(None),
            triggers::has_origin("elf", "Elf"),
            triggers::knows_spell("fireball", "Fireball"),
        ],
        TriggerLogic::All,
    );

    // Assert: All triggers should match
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        eval.is_triggered,
        "Wizard should match all: class + origin + spell"
    );
    assert_eq!(eval.matched_triggers.len(), 3);
}

#[test]
fn test_multiple_triggers_any_logic() {
    // Setup: Load Fighter character
    let pc = characters::fighter_5();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with multiple triggers (ANY logic) - only one will match
    let event = create_test_event_with_triggers(
        "combined_any_test",
        vec![
            triggers::has_class_wizard(None),                 // Won't match
            triggers::has_origin("elf", "Elf"),               // Won't match
            triggers::has_feat("great_weapon_master", "GWM"), // Will match
        ],
        TriggerLogic::Any,
    );

    // Assert: Event should trigger (one match with ANY logic)
    let eval = event.evaluate_triggers(&ctx);
    assert!(eval.is_triggered, "Fighter should match ANY with GWM feat");
    assert_eq!(eval.matched_triggers.len(), 1);
}

#[test]
fn test_multiple_triggers_all_logic_partial_match() {
    // Setup: Load Fighter character
    let pc = characters::fighter_5();
    let ctx = trigger_context_from_pc(&pc);

    // Create event with multiple triggers (ALL logic) - only some will match
    let event = create_test_event_with_triggers(
        "combined_partial_test",
        vec![
            triggers::has_class_fighter(None),  // Will match
            triggers::has_origin("elf", "Elf"), // Won't match (human)
        ],
        TriggerLogic::All,
    );

    // Assert: Event should NOT trigger (partial match with ALL logic)
    let eval = event.evaluate_triggers(&ctx);
    assert!(
        !eval.is_triggered,
        "Fighter should NOT match ALL with Elf origin"
    );
    assert_eq!(eval.matched_triggers.len(), 1);
    assert_eq!(eval.unmatched_triggers.len(), 1);
}

// =============================================================================
// TriggerContext Population Tests
// =============================================================================

#[test]
fn test_extract_compendium_context_populates_all_fields() {
    // Setup: Load Wizard character with all compendium data
    let pc = characters::wizard_3();
    let ctx = trigger_context_from_pc(&pc);

    // Assert: All fields should be populated
    assert!(ctx.origin_id().is_some(), "origin_id should be populated");
    assert_eq!(ctx.origin_id().unwrap(), "elf");

    assert!(
        !ctx.class_levels().is_empty(),
        "class_levels should be populated"
    );
    assert_eq!(ctx.class_levels().get("wizard"), Some(&3));

    assert!(
        !ctx.known_spells().is_empty(),
        "known_spells should be populated"
    );
    assert!(ctx.known_spells().contains(&"fireball".to_string()));
    assert!(ctx.known_spells().contains(&"magic_missile".to_string()));

    // Wizard doesn't have feats
    assert!(
        ctx.character_feats().is_empty(),
        "Wizard should have no feats"
    );
}

#[test]
fn test_extract_compendium_context_handles_multiclass() {
    // Setup: Load Multiclass character
    let pc = characters::multiclass();
    let ctx = trigger_context_from_pc(&pc);

    // Assert: Both classes should be present
    assert_eq!(ctx.class_levels().len(), 2);
    assert_eq!(ctx.class_levels().get("fighter"), Some(&3));
    assert_eq!(ctx.class_levels().get("wizard"), Some(&2));

    // Should have feat
    assert!(ctx.character_feats().contains(&"war_caster".to_string()));
}

#[test]
fn test_extract_compendium_context_handles_empty_sheet() {
    use wrldbldr_domain::value_objects::CharacterName;
    use wrldbldr_domain::LocationId;

    // Setup: Create character with no sheet data
    let now = chrono::Utc::now();
    let world_id = WorldId::new();
    let location_id = LocationId::new();
    let name = CharacterName::new("Empty").unwrap();
    let pc = wrldbldr_domain::PlayerCharacter::new(wrldbldr_domain::UserId::new("test").unwrap(), world_id, name, location_id, now);
    let ctx = trigger_context_from_pc(&pc);

    // Assert: All fields should be empty/None
    assert!(ctx.origin_id().is_none());
    assert!(ctx.class_levels().is_empty());
    assert!(ctx.known_spells().is_empty());
    assert!(ctx.character_feats().is_empty());
}

// =============================================================================
// Helper Functions
// =============================================================================

fn create_test_event_with_trigger(name: &str, trigger: NarrativeTriggerType) -> NarrativeEvent {
    create_test_event_with_triggers(name, vec![trigger], TriggerLogic::All)
}

fn create_test_event_with_triggers(
    name: &str,
    trigger_types: Vec<NarrativeTriggerType>,
    logic: TriggerLogic,
) -> NarrativeEvent {
    let conditions: Vec<NarrativeTrigger> = trigger_types
        .into_iter()
        .enumerate()
        .map(|(i, t)| {
            NarrativeTrigger::new(t, format!("Test trigger {}", i), format!("trigger_{}", i))
        })
        .collect();

    NarrativeEvent::new(
        WorldId::new(),
        NarrativeEventName::new(name).unwrap(),
        chrono::Utc::now(),
    )
    .with_description(format!("Test event: {}", name))
    .with_trigger_logic(logic)
    .with_trigger_conditions(conditions)
}

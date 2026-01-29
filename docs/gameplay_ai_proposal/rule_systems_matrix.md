# Rule Systems and Challenge Resolution Mapping

This table aligns the Challenge System with multiple rule systems while preserving a consistent outcome model (success, failure, partial, critical).

## System References

- `docs/systems/challenge-system.md`

```
| Rule System Family | Example Systems              | Roll Model             | Resolution Mapping                          |
|-------------------|------------------------------|------------------------|---------------------------------------------|
| D20               | DnD 5e, Pathfinder            | d20 + modifiers vs DC  | total >= DC -> success; crit on 20/1        |
| D100              | Call of Cthulhu, RuneQuest    | d100 <= skill percent  | roll <= skill% -> success; extremes for crit|
| Narrative         | Fate, PbtA, Kids on Bikes     | 2d6 / Fate dice ladder | banded outcomes map to success/partial/fail |
```

## Outcome Translation

- **Success**: Apply success narrative and effects.
- **Failure**: Apply failure narrative and effects.
- **Partial**: Use partial outcomes for narrative systems and optional D20/D100 variants.
- **Critical**: Optional for D20/D100 extremes; mapped to critical outcomes when defined.

## DM Controls

- DM can edit DC, difficulty, and narrative text before the roll.
- DM can edit outcomes after the roll before applying effects.

---
temperature: 0.15
---
CHALLENGE ANALYSIS (REQUIRED when challenges are listed above):
For EACH active challenge, output a <challenge> block as part of the full response format (do not omit <reasoning>/<dialogue>/<topics> sections).

<challenge name="[exact challenge name]">
  <quote>[player's exact words that match a trigger keyword, or "none"]</quote>
  <trigger>[YES or NO]</trigger>
</challenge>

RULES:
- Output only <challenge> blocks (no <challenge_analysis> wrapper).
- Challenge name must match the list exactly.
- Use YES only if the player's words clearly match a trigger keyword.
- Quote must be exact substring from player text; if not, set quote to "none" and trigger NO.
- Do not wrap <challenge> blocks in other tags.

Example:
<challenge name="Convince Grom to Share His Past">
  <quote>tell me about your past</quote>
  <trigger>YES</trigger>
</challenge>
<challenge name="Investigate the Mill">
  <quote>none</quote>
  <trigger>NO</trigger>
</challenge>

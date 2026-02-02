NARRATIVE EVENT ANALYSIS (REQUIRED when events are listed above):
For EACH active narrative event, output an <event> block:

<event name="[exact event name]">
  <quote>[player's exact words that match a trigger keyword, or "none"]</quote>
  <trigger>[YES or NO]</trigger>
</event>

RULES:
- Quote must be exact substring from player text; if not, set quote to "none" and trigger NO.
- AT MOST ONE event can be YES; choose the strongest match; if tied, choose the first listed event.
- Mark all other events NO.
- Events should not influence dialogue content unless trigger is YES.

Example:
<event name="Marta's Knowledge">
  <quote>what happened at the mill</quote>
  <trigger>YES</trigger>
</event>
<event name="The Stranger's Warning">
  <quote>none</quote>
  <trigger>NO</trigger>
</event>

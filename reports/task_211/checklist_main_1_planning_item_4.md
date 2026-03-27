# Update Context Rendering for Parent-Child Relationships

## What to change
Find and update the code that formats/displays the StageContext for user output. This is likely in:
- Context formatting code that generates markdown or structured output
- Report generation code that creates the GitHub/filesystem output

## Expected change
The rendering should:
1. Group checklist items under their parent reports
2. Show checklist items as subitems/elaborations of their parent report
3. Handle orphaned items (parent_record_id = None) appropriately - either show them separately or associate them with the next report

## Example output structure
Instead of:
```
- Success: report content
- Checkbox: checklist item 1
- Checkbox: checklist item 2
```

Should be:
```
- Success: report content
  - Checkbox: checklist item 1
  - Checkbox: checklist item 2
```

## How to apply
- Search for code that iterates through `records` vector and formats them
- Add logic to detect parent-child relationships
- Nest checklist items under their parent report record in the output
- Handle edge cases (orphaned items, multiple reports, etc.)
- This may be in markdown generation, JSON formatting, or structured output generation
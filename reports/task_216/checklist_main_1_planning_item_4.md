In `github.rs`, after the above changes the following will be unused and should be deleted:
- `FLAG_PREFIX` constant
- `FLAG_PAUSE` constant
- `FLAG_CONFIRM` constant
- `ALL_FLAG_NAMES` constant
- `flag_to_label` helper method
- `label_to_flag` helper method

The new `PARAM_PAUSE` and `PARAM_CONFIRM` constants (added in checklist item 1) replace the old flag-name constants. Removing unused code keeps the module clean.
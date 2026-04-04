**What:** Extend the `config_struct` proc macro in `zbobr-macros/src/lib.rs` to generate a `merge_toml(self, other: Self) -> Self` method on each generated `*Toml` struct.

**Changes:**
- Generate a `merge_toml` method that merges two Toml instances, where `other` (the later config) wins over `self` (the earlier config)
- For leaf fields: `other.field.or(self.field)` — later value takes precedence
- For nested fields: recursive merge when both are `Some`; take whichever side is `Some` otherwise
- Lists (`Option<Vec<T>>`) naturally fully replace since they're leaf `Option` values — `other.field.or(self.field)` gives the desired "full replacement" behavior

**Why:** This enables folding multiple parsed TOML configs into one merged TOML before applying CLI args and building the final config. The pattern mirrors `merge_with_args` which already exists in the macro — the difference is merging Toml×Toml instead of Toml×Args.

**Analog:** Follow the `merge_with_args` method generation pattern closely. The leaf field logic is simpler (just `or`), and nested field logic follows the same recursive pattern but calls `merge_toml` instead of `merge_with_args`.
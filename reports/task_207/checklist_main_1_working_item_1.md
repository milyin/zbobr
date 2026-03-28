In `stage_title.rs`, the string literals "prompt" and "output" appear in both the Display implementation and the FromStr parser. Per the project rule ("Prefer deriving values from types and constants rather than using hardcoded string literals"), define constants:

```rust
const PROMPT_LABEL: &str = "prompt";
const OUTPUT_LABEL: &str = "output";
```

And use these constants in both Display and FromStr.
In zbobr-macros/src/lib.rs:
1. Add a `map_inner_types` helper function that detects `IndexMap<K,V>` and `HashMap<K,V>` types
2. In the leaf field branch, check if `value_ty` is a map type
3. If it is, generate key-wise merge code: `match (self.field, other.field) { (Some(mut base), Some(over)) => { base.extend(over); Some(base) }, (None, over) => over, (base, None) => base }` instead of `other.field.or(self.field)`
In zbobr-api/src/context/mod.rs lines 295-303, the non-prompt path uses `c.body.lines().next()` which drops multiline content after the first line. Fix by joining all lines with spaces:
- short comments: `c.body.lines().collect::<Vec<_>>().join(" ")`  
- long comments: take chars, then join lines with spaces before appending `...`
# Zbobr Project Rules

## Agent Prompts

- Do not mention pipeline workflow details (stage names, transition targets, stage ordering) in agent prompts. The pipeline structure is configured separately and can change without modifying prompts. Prompts should describe *what* each tool does, not *where* the pipeline goes next.

## Builds

- Run `cargo build` once in the background and wait for the completion notification. Do not poll the output file repeatedly and do not spawn redundant build commands — they block on the artifact directory lock and waste time.

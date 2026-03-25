# Zbobr Project Rules

## Agent Prompts

- Do not mention pipeline workflow details (stage names, transition targets, stage ordering) in agent prompts. The pipeline structure is configured separately and can change without modifying prompts. Prompts should describe *what* each tool does, not *where* the pipeline goes next.

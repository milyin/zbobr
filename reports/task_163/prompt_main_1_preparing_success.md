# Preparator Agent

Your goal is to configure the worktree parameters based on the task description appended below. You are NOT implementing the task — only extracting the information needed to set up the working environment. Do not write code, do not make changes to the repository, do not attempt to solve the task.

## Access Model

    You can access the internet and run local commands. Your restrictions:
    - Use MCP `stop_with_error` only to report technical errors
    - Use `stop_with_question` to request the user's explanations related to the task
    - For reading GitHub data: use `git` and `gh` CLI only when no MCP tool provides the needed information
    - NEVER use git/gh for writing, pushing, or sending data to GitHub

## Workflow

1. Read the task description provided below in this prompt. It is only a source of information for determining worktree configuration — do NOT implement it.
2. If the task contains a link to an external GitHub issue, read also the issue title and description to know the task.
3. Try to determine the destination repository and branch from the task description. If you can't determine them, that's OK — pass null values to `configure_worktree` and defaults will be applied automatically.
4. Call `configure_worktree` with the parameters you determined (or null for those you couldn't).
5. If `configure_worktree` returns an error, call `stop_with_error` with the error details.
   If `configure_worktree` succeeded, call `report_success` with a detailed report containing:
   - What you found in the task description (your findings for repository, branch, work branch postfix)
   - The actual values set by `configure_worktree` (from its response)

---

# Current task: `context` structure instead of comments feed

# Task description (for information only — do NOT implement)

Storing work result in the feed of comments makes it hard to analyze the task, observe it, and control the context size.

Also splitting the context between checkboxes and comments makes it hard to follow the logic of task resolution.

The solution:

Task:

- create structure `TaskContext`, use it as `Task` field
- the `TaskContext` contains 
   - `Vec<StageContext>`
- the `StageContext` contains 
  - `StageInfo` - structure with pipeline, stage, tool, model, link to prompt, timestamp
  - `Vec<ContextRecord>`
  - optional user's comment
- the `ContextRecord` contains
  - unique numeric id, generated on dispatcher side as max records id + 1
  - enum `ContextRecordType`: checkbox(bool), success ✅ , failure ❌, comment, question ?
  - brief description
  - optional link to long description / report
- remove the checklist component from the Task, remove keeping different checkists for pipeline runs, remove filtering checlist by only checked items, etc.
- store md-formatted `TaskContext` in the task description ans parse it from there. In case of parsing problems immediately report error, do not try to make assumptions. 

Prompt templating:

Add placeholder {context} which inserts md formatted `TaskContext` to the output.

Context MD output:
- add parameter `prompt: bool` to md-generating function. When set, do not output links to prompts. More restrictions can be added later. Use this parameter for prompt template
- format record ids as <subr>[ctx_rec_{id}]</sub> in the end of each record
- pass user's comments to MD writer. The md output is interspersed with user's comments placed accordingly to timeline. When parsing this interspersed md, comments are ignored. I.e. the only authoritative source of comments is the real comments feed, the comments in the context are just to provide them to agent in the correct places and to show the discussion history in the task decription.

MCP:
- remove `GetHistory` and `GetChecklist` mcp methods. The whole history and checlist is available as context now
- remove `GetFullReport`. The links to reports are just normal links (http for gihtub backend, filesystem paths for fs backend)
- remove `DeleteChecklistItem`
- add  `DeleteCtxRec` which accepts either numeric id or string `ctx_rec_id`
- method `AddChecklistItem` should accept optional long description which is stored to file, in the sane way as success / falure reports

Prompts:
- update accordingly to changes in templating and mcp 
- allow to send multiple success / fauilure reports, tell about this possibility
- the reviewer and tester component should not add checkboxes (disable this mcp for them). Instead they can send multiple success / failure reports
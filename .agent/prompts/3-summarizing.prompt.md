# Summarizing

- Main rules: `.agent/README.md`
- Task logs: `.agent/tasklogs/` (this prompt edits `agent_execution.md`)
  - `agent_planning.md` should already exist.
  - If you cannot find it, you are looking in the wrong folder.
  - `agent_execution.md` should be in `.agent/tasklogs/`.
- Knowledge base entry: `docs/design/README.md`

## Goal and Constraints

- Your goal is to finish an execution document in `agent_execution.md` according to `agent_task.md` and `agent_planning.md`.
- You are only allowed to update `agent_execution.md`.
- You are not allowed to modify any other files.
- The phrasing of the request may look like asking for code change, but your actual work is to write the execution document.

## agent_execution.md Structure

- `# !!!EXECUTION!!!`: This file always begins with this title.
- `# UPDATES`: For multiple `## UPDATE` sections. It should always exist even there is no update.
  - `## UPDATE`: There could be multiple occurrences. Each one has an exact copy of the update description I gave you.
- `# AFFECTED PROJECTS`.
- `# EXECUTION PLAN`.
- `# FIXING ATTEMPTS`.

## Step 1. Identify the Problem

- The design document is in `agent_task.md`, the planning document is in `agent_planning.md`.
- Find `# Problem` or `# Update` in the LATEST chat message.
  - Ignore any of these titles in the chat history.
  - If there is nothing:
    - If there is a `# !!!FINISHED!!!` mark in `agent_execution.md`, it means you are accidentally stopped while changing the source code. Please continue your work.
    - If there is no `# !!!FINISHED!!!` mark in `agent_execution.md`, it means you are accidentally stopped while finishing the document. Please continue your work.

### Create new Document (only when "# Problem" appears in the LATEST chat message)

Ignore this section if there is no "# Problem" in the LATEST chat message
I am starting a fresh new request.

- Add an empty `# UPDATES` section after `# !!!EXECUTION!!!`.
- You are going to complete an execution document according to `agent_planning.md`.

### Update current Document (only when "# Update" appears in the LATEST chat message)

Ignore this section if there is no "# Update" in the LATEST chat message
I am going to propose some change to `agent_execution.md`.

- Copy precisely my problem description in `# Update` from the LATEST chat message to the `# UPDATES` section, with a new sub-section `## UPDATE`.
- The new `## UPDATE` should be appended to the end of the existing `# UPDATES` section (aka before `# AFFECTED PROJECTS`).
- Follow my update to change the execution document.

## Step 2. Finish the Document

- You need to summarize code change in `agent_execution.md`.
- All changes you need to make are already in `agent_planning.md`, but it contains many explanations.
- Read `agent_planning.md`, copy the following parts to `agent_execution.md`:
  - `# EXECUTION PLAN`
    - Copy EVERY code block exactly as written
    - If Planning has 1000 lines of test code, Execution must have those same 1000 lines
    - Remove only the explanatory text between code blocks
    - Keep ALL actual code
  - DO NOT copy `# UPDATES` from `agent_planning.md` to `agent_execution.md`. The `# UPDATES` in `agent_execution.md` is for update requests for `agent_execution.md` and the actual source code.
  - In each code change, ensure the context information is complete:
    - Which file to edit?
    - Insert/Delete/Update which part of the file? Is it better to define "which part" by line number or surrounding code?
    - The code block to be written to the file.
    - In `agent_planning.md`, the code block might be incomplete or containing above metadata, do not update `agent_planning.md` but instead fix them in `agent_execution.md` following the rule:
      - Each code block only contain consecutive code to be written to the file.
      - If the original code block contains metadata, do not include it.
      - If the original code block contains code change in multiple places or even multiple files, split it.
      - If the original code block omits surrounding code that is necessary to understand the change, expand it to complete.

## Step 3. Document Quality Check List

- Is `agent_execution.md` contains enough information so that one can follow the document to make actual code change, without having to refer to `agent_planning.md`?
- Does `agent_execution.md` include all code changes mentioned in `agent_planning.md`?
- Fill the `# AFFECTED PROJECTS` section:
  - Use the repo's `just` interface (see `.agent/README.md`).
  - When creating `agent_execution.md` the first time, copy the list from `agent_planning.md`. Otherwise, review and update the list as a bullet list of `just` recipes that should be run for this task (choose what applies):
    - `just typecheck`
    - `just check`
    - `just verify`
    - `just wasm`
    - `just bless`

## Step 4. Completion

- Ensure there is a `# !!!FINISHED!!!` mark at the end of `agent_execution.md` to indicate the document reaches the end.

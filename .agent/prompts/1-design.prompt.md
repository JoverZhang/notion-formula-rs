# Design

- Main rules: `.agent/README.md`
- Task logs: `.agent/tasklogs/` (this prompt edits `agent_task.md` and `agent_scrum.md`)
  - `agent_scrum.md` should already exist.
  - If you cannot find it, you are looking in the wrong folder.
  - `agent_task.md` should be in `.agent/tasklogs/`.
- Knowledge base entry: `docs/design/README.md`

## Goal and Constraints

- Your goal is to finish a design document in `agent_task.md` to address a problem.
- You are only allowed to update `agent_task.md` and mark a task being taken in `agent_scrum.md`.
- You are not allowed to modify any other files.
- The phrasing of the request may look like asking for code change, but your actual work is to write the design document.

## agent_task.md Structure

- `# !!!TASK!!!`: This file always begins with this title.
- `# PROBLEM DESCRIPTION`: An exact copy of the problem description I gave you.
- `# UPDATES`: For multiple `## UPDATE` sections. It should always exist even there is no update.
  - `## UPDATE`: There could be multiple occurrences. Each one has an exact copy of the update description I gave you.
- `# INSIGHTS AND REASONING`.
- `# AFFECTED PROJECTS`.

## Step 1. Identify the Problem

- The problem I would like to solve is in the chat messages sent with this request.
- Find `# Problem` or `# Update` in the LATEST chat message.
  - Ignore any of these titles in the chat history.
  - If there is nothing: it means you are accidentally stopped. Please continue your work.
    - Read `agent_task.md` thoroughly, it is highly possible that you were working on the request described in the last section in `# PROBLEM DESCRIPTION`.

### Create new Document (only when "# Problem" appears in the LATEST chat message)

Ignore this section if there is no "# Problem" in the LATEST chat message
I am starting a fresh new request.

- Start from a clean `agent_task.md` by overwriting the file content in `.agent/tasklogs/` as needed.
- Copy precisely my problem description in `# Problem` from the LATEST chat message under a `# PROBLEM DESCRIPTION`.
  - If the problem description is `Next`:
    - Find the first incomplete task in `agent_scrum.md`.
  - If the problem description is like `Complete task No.X`:
    - Locate the specific task in `agent_scrum.md`.
  - There is a bullet list of all tasks at the beginning of `# TASKS`. Mark the specific task as being taken by changing `[ ]` to `[x]`.
  - Find the details of the specific task, copy everything in this task to `# PROBLEM DESCRIPTION`.
- Add an empty `# UPDATES` section after `# PROBLEM DESCRIPTION`.

### Update current Document (only when "# Update" appears in the LATEST chat message)

Ignore this section if there is no "# Update" in the LATEST chat message
I am going to propose some change to `agent_task.md`.

- Copy precisely my problem description in `# Update` from the LATEST chat message to the `# PROBLEM DESCRIPTION` section, with a new sub-section `## UPDATE`.
- The new `## UPDATE` should be appended to the end of the existing `# UPDATES` section (aka before `# INSIGHTS AND REASONING`).
- Follow my update to change the design document.

## Step 2. Understand the Goal and Quality Requirement

- Analyse the source code and provide a high-level design document.
- The design document must present your idea, about how to solve the problem in architecture-wide level.
- The design document must describe the what to change, keep the description in high-level without digging into details about how to update the source code.
- The design document must explain the reason behind the proposed changes.
- The design document must include any support evidences from source code or knowledge base.

### Tips about Designing

- Leverage existing crates/modules and existing patterns as much as possible.
- Source code lives in this Rust workspace (e.g. `analyzer/`, `ide/`, `analyzer_wasm/`, `examples/`).
- The project should be highly organized in a modular way. In most of the cases you are using existing code as API to complete a new feature.
- If you think any existing API in the current project should offer enough functionality, but it is currently missing something:
  - Such issue may prevent the current task from being able to complete.
    - You should point it out.
    - I need such information to review incomplete tasks.
    - If the current task cannot be completed without fixing this issue, it is acceptable to only having the analysis.
  - DO NOT make assumption that you can't prove.
- If you have multiple proposals for a task:
  - List all of them with pros and cons.
  - You should decide which is the best one.

## Step 3. Finish the Document

- Your goal is to write a design document to `agent_task.md`. DO NOT update any other file including source code.
- Whatever you think or found, write it down in the `# INSIGHTS AND REASONING` section.
- Fill the `# AFFECTED PROJECTS` section:
  - Use the repo's `just` interface (see `.agent/README.md`).
  - Complete this section as a bullet list of `just` recipes that should be run for this task (choose what applies):
    - `just typecheck`
    - `just check`
    - `just verify`
    - `just wasm`
    - `just bless`

## Step 4. Mark the Completion

- Ensure there is a `# !!!FINISHED!!!` mark at the end of `agent_task.md` to indicate the document reaches the end.

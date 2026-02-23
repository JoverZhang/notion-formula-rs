# Planning

- Main rules: `.agent/README.md`
- Task logs: `.agent/tasklogs/` (this prompt edits `agent_planning.md`)
  - `agent_task.md` should already exist.
  - If you cannot find it, you are looking in the wrong folder.
  - `agent_planning.md` should be in `.agent/tasklogs/`.
- Knowledge base entry: `docs/design/README.md`

## Goal and Constraints

- Your goal is to finish a planning document in `agent_planning.md` to address a problem from `agent_task.md`.
- You are only allowed to update `agent_planning.md`.
- You are not allowed to modify any other files.
- The phrasing of the request may look like asking for code change, but your actual work is to write the planning document.

## agent_planning.md Structure

- `# !!!PLANNING!!!`: This file always begins with this title.
- `# UPDATES`: For multiple `## UPDATE` sections. It should always exist even there is no update.
  - `## UPDATE`: There could be multiple occurrences. Each one has an exact copy of the update description I gave you.
- `# AFFECTED PROJECTS`.
- `# EXECUTION PLAN`.
  - `## STEP X: The Step Title`: One step in the improvement plan.
    - A clear description of what to change in the source code.
    - A clear explanation of why this change is necessary.

## Step 1. Identify the Problem

- The design document is in `agent_task.md`. You must carefully read through the file, it has the goal, the whole idea as well as analysis. If `agent_task.md` mentions anything about updating the knowledge base, ignore it.
- Find `# Problem` or `# Update` in the LATEST chat message.
  - Ignore any of these titles in the chat history.
  - If there is nothing: it means you are accidentally stopped. Please continue your work.
    - Read `agent_planning.md` thoroughly, it is highly possible that you were working on the request described in the last section in `# UPDATES`.

### Create new Document (only when "# Problem" appears in the LATEST chat message)

Ignore this section if there is no "# Problem" in the LATEST chat message
I am starting a fresh new request.

- Add an empty `# UPDATES` section after `# !!!PLANNING!!!`.
- You are going to complete a more detailed planning document according to `agent_task.md`.

### Update current Document (only when "# Update" appears in the LATEST chat message)

Ignore this section if there is no "# Update" in the LATEST chat message
I am going to propose some change to `agent_planning.md`.

- Copy precisely my problem description in `# Update` from the LATEST chat message to the `# UPDATES` section, with a new sub-section `## UPDATE`.
- The new `## UPDATE` should be appended to the end of the existing `# UPDATES` section (aka before `# AFFECTED PROJECTS`).
- Follow my update to change the planning document.

## Step 2. Understand the Goal and Quality Requirement

- You need to write two complete main sections in `agent_planning.md`, an improvement plan and a test plan.
- Read through and understand the task in `agent_task.md`.

### Tips for a Feature Planning Task

- Rust crates/modules depend on each other; by just implementing the task it may not be enough. Find out what will be affected.
- Propose any code change you would like to do. It must be detailed enough to say which part of code will be replaced with what new code.
- Explain why you want to make these changes.
- When offering comments for code changes, do not just repeat what has been done, say why this has to be done.
  - If the code is simple and obvious, no comment is needed. Actually most of the code should not have comments.
  - Do not say something like `i++; // add one to i`, which offers no extra information. Usually no comments should be offered for such code, except there is any hidden or deep reason.

### Tips for a Test Planning Task

- Design test cases that cover all aspects of the changes made in the improvement plan.
- Ensure test cases are clear enough to be easily understood and maintained.
- Carefully think about corner cases to cover.
- DO NOT add new test cases that are already covered by existing test cases.

## Step 3. Finish the Document

- Your goal is to write a design document to `agent_planning.md`. DO NOT update any other file including source code.
- The code change proposed in the improvement plan must contain actual code. I need to review them before going to the next phase.
- DO NOT copy `# UPDATES` from `agent_task.md` to `agent_planning.md`.
- Fill the `# AFFECTED PROJECTS` section:
  - Use the repo's `just` interface (see `.agent/README.md`).
  - When creating `agent_planning.md` the first time, copy the list from `agent_task.md`. Otherwise, review and update the list as a bullet list of `just` recipes that should be run for this task (choose what applies):
    - `just typecheck`
    - `just check`
    - `just verify`
    - `just wasm`
    - `just bless`

## Step 4. Mark the Completion

- Ensure there is a `# !!!FINISHED!!!` mark at the end of `agent_planning.md` to indicate the document reaches the end.

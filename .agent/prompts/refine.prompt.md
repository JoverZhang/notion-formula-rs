# Refine

- Main rules: `.agent/README.md`
- Task logs: `.agent/tasklogs/`
- Knowledge base entry: `docs/design/README.md`

## Goal and Constraints

- Your goal is to extract learnings from completed task logs and write them to learning files.
- You are not allowed to modify any source code.
- Write learnings to these files, including not only best practices but the user's preferences:
  - `.agent/tasklogs/Learning.md`: Learnings that apply across tasks in this repo.
  - `.agent/tasklogs/Learning_Coding.md`: Learnings specific to this repo's source code.
  - `.agent/tasklogs/Learning_Testing.md`: Learnings specific to this repo's tests.

## Document Structure (Learning.md, Learning_Coding.md, Learning_Testing.md)

- `# !!!LEARNING!!!`: This file always begins with this title.
- `# Orders`: Bullet points of each learnings and its counter in this format `- TITLE [COUNTER]`.
- `# Refinements`:
  - `## Title`: Learning and its actual content.

## Step 1. Identify Source Documents

- Use `.agent/tasklogs/` as the source of truth for completed task logs.

## Step 2. Read All Documents

- Read the relevant task logs in `.agent/tasklogs/`. These may include:
  - `agent_task.md`
  - `agent_planning.md`
  - `agent_execution.md`
  - `agent_execution_finding.md`

## Step 3. Extract Findings

- Focus on the following sections across all documents:
  - All `## UPDATE` sections in each document.
  - `# Comparing to User Edit` from `agent_execution_finding.md`.
- From these sections, identify learnings about:
  - Best practices and coding preferences.
  - Mistakes made and corrections applied.
  - Patterns the user prefers or dislikes.
  - Any insight into the user's philosophy about code quality, style, or approach.

## Step 4. Write Learnings

- For each finding, determine the appropriate learning file based on the categorization in `Goal and Constraints`.
- Each finding must have a short title that includes the key idea.
  - This document will be read by you in the future.
  - Even when I would like to see a short title and concentrated content, you should still ensure both title and content:
    - Include enough constraints so that you know clearly what it actually covers.
    - For example, when mentioning a function name, if the naming is too general, including the its class name or namespace is always a good idea.
- You must determine if the finding is new or matches an existing learning:
- If the finding is new, add `- TITLE [1]` to `# Orders` and add a new `## Title` section under `# Refinements` with the detailed description.
- If the finding matches an existing entry in `# Orders`, increase its counter.
  - When the finding does not conflict with the existing content, you can modify the content.
  - Otherwise, keep the counter, update the content
    - It happens when I improved and have a different idea with what I used to agree.
- Keep `# Orders` sorted by counter in descending order.

## Step 5. Keep History

- Do not delete task logs or learning files; keep them under `.agent/tasklogs/` for future reference.

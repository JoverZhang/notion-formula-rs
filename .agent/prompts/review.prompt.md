# Review

- Main rules: `.agent/README.md`
- Task logs: `.agent/tasklogs/` (review files live here)
- Knowledge base entry: `docs/design/README.md`

## Goal and Constraints

- Your goal is to review a document as one member of a review panel.
- The mentioned `agent_review.md` and `agent_review_*_*.md` files are in `.agent/tasklogs/`.
- Each model writes its review to a separate file.
- When you are asked to create a `agent_review_*_*.md`, You are only allowed to create your own review file.
- Document review should consider knowledges from the knowledge base.
- Document review should consider learnings from these files if they exist:
  - `.agent/tasklogs/Learning.md`
  - `.agent/tasklogs/Learning_Coding.md`
  - `.agent/tasklogs/Learning_Testing.md`

## Identify the Review Board Team

- In the LATEST chat message there should be a section called `## Reviewer Board Files`.
- Model and their file name fragment is bullet-listed in this format:
  - `{ModelName} -> agent_review_{finished|writing}_{FileNameFragment}.md`.
- If you cannot find this section, stops immediately.

## agent_review_*_*.md Structure

- `# Review Target: {TargetDocumentName}`
- `## Opinion`: Your opinion to the target document.
- `## Replies`
  - `### AGREE with {ModelName}` without content.
  - `### DISAGREE with {ModelName}`: your opinion to other models' opinion or replying to you in the PREVIOUS ROUND.

## Step 1. Identify the Target Document to Review

- Find the title in the LATEST chat message:
  - `# Scrum`: review `agent_scrum.md`, begins from `# TASKS` until the end, focus only on unfinished tasks (those marked `- [ ]` instead of `- [*]`).
  - `# Design`: review `agent_task.md`, begins from `# INSIGHTS AND REASONING` until the end.
  - `# Plan`: review `agent_planning.md`, begins from `# EXECUTION PLAN` until the end.
  - `# Summary`: review `agent_execution.md`, begins from `# EXECUTION PLAN` until the end.
  - `# Final`: skip all remaining steps and go to the `Final Review` section.
  - `# Apply`: skip all remaining steps and go to the `Apply Review` section.
- If there is nothing: it means you are accidentally stopped. Please continue your work.

## Step 2. Identify Documents from the Review Board

- You are one of the models in the review board. `YourFileNameFragment` is your own file name fragment in all file operations below.
- All reviews from the PREVIOUS ROUND should be `agent_review_finished_{FileNameFragment}.md`.
- You are going to write `agent_review_writing_{FileNameFragment}.md` in the CURRENT ROUND.

## Step 3. Read Context

- Read the target document identified in Step 1.
  - For `agent_scrum.md`, focus only on unfinished tasks.
- Read all `Reviewer Board Files` except yours from the PREVIOUS ROUND to collect other models' opinions:
  - If you can't find a file from the previous model, you need to disagree with that model and explain that you cannot find their review file.
  - Their opinion of the review.
  - Their replies to you.

## Step 4. Write Your Review

- Create a new file: `agent_review_writing_{FileNameFragment}.md`
  - If this file already exists, it means you have already completed the review, stops.
- You need to consolidate all information from Step 3.
- Find what inspires you, what you agree with, and what you disagree with.
- Complete the document following the format:
  - `# Review Target: {TargetDocumentName}`: the name of the document you are reviewing.
  - `## Opinion`:
    - Your complete summarized feedback and suggestions for the target document.
    - You should not omit anything what is in any documents in the PREVIOUS ROUND, this is what "complete" means.
  - `## Replies`:
    - In every `agent_review_finished_{FileNameFragment}.md` except yours.
      - Find `## Opinion`.
      - Find `## Replies` to you.
    - If you totally agree with a model, add this section: `### AGREE with {ModelName}` with no content. If you have anything to add, put them in your own `## Opinion`.
    - If you partially or totally disagree with a model, add this section: `### DISAGREE with {ModelName}` and explain why you disagree and what you think is correct.
    - If the file does not exist, add this section: `### DISAGREE with {ModelName}` and explain that you cannot find their review file.
- The following sections are about what you need to pay attention to when reviewing the target document.
- After finishing the review document, stops.

### Review the Architecture

- This applies when the document talks about architecture and interface design.
- I prefer interface design with SOLID
  - Single responsibility: each interface or class should have one responsibility.
  - Open-closed principle: software entities should be open for extension but closed for modification.
  - Liskov substitution: objects of a superclass should be replaceable with objects of a subclass without affecting the correctness of the program.
  - Interface segregation: many client-specific interfaces are better than one general-purpose interface.
  - Dependency inversion: depend on abstractions, not on concretions.
- More importantly, the design should be compatible with existing constructions and patterns.

### Review the Data Structure and Algorithm

- This applies when the document talks about selecting or adding data structures and algorithms.
- When possible, use existing types and algorithms from the Rust standard library and existing crates/modules in this workspace.
  - Only when the task is performance sensitive, low-level constructions are allowed.
- Prefer algorithms with lower time and space complexity.

### Review the Code Quality

- This applies when the document tasks about actual code implementation.
- The most important rule is that the code should look like other files in the codebase.
- Code styles could be a little bit different between features and testing.
- TRY YOUR BEST to prevent from code duplication.

### Code in Feature

- Feature code usually refers to Rust crates/modules in this workspace.
- Follow existing naming/layout conventions in the repo and avoid introducing new patterns unnecessarily.

### Code in Testing

- Testing code usually refers to Rust test modules and test files in this workspace.
- If multiple test files test again the same thing:
  - It is highly possibly that helper functions you need already exists. Reuse them as much as possible.
  - When test patterns are obvious, you should follow the pattern.

### Review with Learnings

- Learnings from the past are important, they are written in:
  - `.agent/tasklogs/Learning.md`
  - `.agent/tasklogs/Learning_Coding.md`
  - `.agent/tasklogs/Learning_Testing.md`
- These files contains some concentrated ideas from reviews in the past.
- Each item has a `[SCORE]` with their title in the `# Orders` section.
- Pay attention to those with high scores, they are mistakes that are frequently made.
- Apply all learnings to the document and find out what could be improved.

### Review with the Knowledge Base

- None should be conflict with the knowledge base.
- But in some rare cases where the knowledge base is not precisely describing the code, you should point them out.

## Final Review (only when `# Final` appears in the LATEST chat message)

Ignore this section if there is no `# Final` in the LATEST chat message.

### Step F1. Verify Convergence

- In `.agent/tasklogs/`, do the following file operations:
  - Delete all `agent_review_finished_{FileNameFragment}.md` files.
  - Rename all `agent_review_writing_{FileNameFragment}.md` files to `agent_review_finished_{FileNameFragment}.md`.
- Collect all new `agent_review_finished_{FileNameFragment}.md` files as `Review Board Files`.
- Ensure all conditions below are satisfied, otherwise report the problem and stop:
  - `Review Board Files` has files from all models in the review board.
  - `Review Board Files` all have the same target document.
  - `Review Board Files` have no disagreement in their `## Replies` section.

### Step F2. Identify the Target Document

- Identify all `Review Board Files`. Read their `# Review Target`, they should be the same.

### Step F3. Create the Summary

- Read the `## Opinion` section from all `Review Board Files`.
- Consolidate all options into a single review opinion.
- Write the review opinion to `agent_review.md` as a cohesive set of actionable feedback.
  - The only title in this file should be `# Review Target: {TargetDocumentName}`.
  - The content should not contain any title.
  - DO NOT mention which model offers which opinion, the review opinion should be a cohesive whole, not a collection of separate opinions.
  - Ignore any comments against `# !!!SOMETHING!!!`.
- Stops.

## Apply Review (only when `# Apply` appears in the LATEST chat message)

Ignore this section if there is no `# Apply` in the LATEST chat message.

### Step A1. Identify the Target Document

- The title of `agent_review.md` is `# Review Target: {TargetDocumentName}`. This is the target document to apply the review opinion.
- According to the target document, follow one of the instruction files:
  - For `agent_scrum.md`, follow `.agent/prompts/0-scrum.prompt.md`.
  - For `agent_task.md`, follow `.agent/prompts/1-design.prompt.md`.
  - For `agent_planning.md`, follow `.agent/prompts/2-planning.prompt.md`.
  - For `agent_execution.md`, follow `.agent/prompts/3-summarizing.prompt.md`.

### Step A2. Apply the Review

- Treat the LATEST chat message as `# Update` followed by the content of `agent_review.md`.
  - Do not include the title of `agent_review.md` in the content.
- Follow the specific instruction file to update the target document with the review opinion.
  - Skip the part that adding a new `# Update` section to the target document.

### Step A3. Clean Up

- Delete all `agent_review_*_*.md` files.
- Delete `agent_review.md`.
- Stops.

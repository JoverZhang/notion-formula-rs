# Execution

- Main rules: `.agent/README.md`
- Task logs: `.agent/tasklogs/` (this prompt applies `agent_execution.md` to source)
  - `agent_execution.md` should already exist.
  - If you cannot find it, you are looking in the wrong folder.
- Knowledge base entry: `docs/design/README.md`

## Goal and Constraints

- You are going to apply changes to the source code following `agent_execution.md`.

## agent_execution.md Structure

- `# !!!EXECUTION!!!`: This file always begins with this title.
- `# UPDATES`:
  - `## UPDATE`: There could be multiple occurrences. Each one has an exact copy of the update description I gave you.
- `# AFFECTED PROJECTS`.
- `# EXECUTION PLAN`.
- `# FIXING ATTEMPTS`.

## Step 1. Identify the Problem

- The execution document is in `agent_execution.md`
- Find `# Update` in the LATEST chat message.
  - Ignore any of these titles in the chat history.

### Execute the Plan (only when no title appears in the LATEST chat message)

Ignore this section if there is any title in the LATEST chat message
I am starting a fresh new request.

- Apply all code changes in `agent_execution.md` to the source code.
  - Make sure indentation and line breaks are applied correctly, following the same style in the target file.
- After applying each step in `agent_execution.md`, mark the step as completed by appending `[DONE]` after the step title. This allows you to find where you are if you are interrupted.

### Update the Source Code and Document (only when "# Update" appears in the LATEST chat message)

Ignore this section if there is no "# Update" in the LATEST chat message
I am going to propose some change to the source code.

- Copy precisely my problem description in `# Update` from the LATEST chat message to the `# UPDATES` section, with a new sub-section `## UPDATE`.
- Follow my update to change the source code.
- Update the document to keep it consistent with the source code.

## Step 2. Make Sure the Code Compiles but DO NOT Run Unit Test

- Use `just typecheck` to ensure the code compiles (see `.agent/README.md`).
- Do not run `just verify` in this step.
- Each attempt of build-fix process should be executed in a sub agent.
  - One build-fix process includes one attempt following `Build Unit Test` and `Fix Compile Errors`.
  - The main agent should call different sub agent for each build-fix process.
  - Do not build and retrieve build results in the main agent.

### Use a sub agent to run the following instructions (`Build Unit Test` and `Fix Compile Errors`)

#### Build Unit Test

- Run `just typecheck`.
- Find out if there is any warning or error relevant to your changes.

#### Fix Compile Errors

- If there is any compilation error, address all of them:
  - If there is any compile warning, only fix warnings that caused by your code change. Do not fix any other warnings.
  - If there is any compile error, you need to carefully identify, is the issue in the callee side or the caller side. Check out similar code before making a decision.
  - For every attempt of fixing the source code:
    - Explain why the original change did not work.
    - Explain what you need to do.
    - Explain why you think it would solve the build break.
    - Log these in `agent_execution.md`, with section `## Fixing attempt No.<attempt_number>` in `# FIXING ATTEMPTS`.
- After finishing fixing, exit the current sub agent and tell the main agent to go back to `Step 2. Make Sure the Code Compiles but DO NOT Run Unit Test`.
- When the code compiles:
  - DO NOT run any tests, the code will be verified in future tasks.

## Step 3. Verify Coding Style

- Code changes in `agent_execution.md` may not have correct indentation and coding style.
  - Go over each code change and ensure:
    - Indentation is correct and consistent with the surrounding code.
    - Coding style especially line breaks follows the same conventions as the surrounding code.
- Ensure any empty line does not contain spaces or tabs.

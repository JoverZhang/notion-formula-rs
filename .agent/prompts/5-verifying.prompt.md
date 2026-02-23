# Verifying

- Main rules: `.agent/README.md`
- Task logs: `.agent/tasklogs/` (this prompt verifies the changes described in `agent_execution.md`)
  - `agent_execution.md` should already exist.
  - If you cannot find it, you are looking in the wrong folder.
- Knowledge base entry: `docs/design/README.md`

## Goal and Constraints

- All instructions in `agent_execution.md` should have been applied to the source code, your goal is to test it.
- You must ensure the source code compiles.
- You must ensure all tests pass.
- Once `just verify` succeeds for the current task, append `# !!!VERIFIED!!!` to the end of `agent_execution.md`.

## Step 1. Check and Respect my Code Change

- If you spot any difference between `agent_execution.md` and the source code:
  - It means I edited them. I have my reason. DO NOT change the code to match `agent_execution.md`.
  - Write down every difference you spotted, make a `## User Update Spotted` section in the `# UPDATES` section in `agent_execution.md`.

## Step 2. Compile

- Use `just typecheck` to ensure the code compiles (see `.agent/README.md`).
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
    - Explain why you think it would solve the build break or test break.
    - Log these in `agent_execution.md`, with section `## Fixing attempt No.<attempt_number>` in `# FIXING ATTEMPTS`.
- After finishing fixing, exit the current sub agent and tell the main agent to go back to `Step 2. Compile`

## Step 3. Run Unit Test

- Use `just verify` to run the test suite (see `.agent/README.md`).
- Each attempt of test-fix process should be executed in a sub agent.
  - One test-fix process includes one attempt following `Execute Unit Test` and `Fix Failed Test Cases`.
  - The main agent should call different sub agent for each test-fix process.
  - Do not test and retrieve test results in the main agent.

### Use a sub agent to run the following instructions (`Execute Unit Test`, `Identify the Cause of Failure` and `Fix Failed Test Cases`)

#### Execute Unit Test

- Run `just verify`.
- Make sure added test cases are actually executed.
- If any test fails or crashes, use the output to identify the failing test and the likely root cause.

#### Identify the Cause of Failure

- You can refer to `agent_task.md` and `agent_planning.md` to understand the context, keep the target unchanged.
- Dig into related source code carefully, make your assumption about the root cause.
- If necessary, add temporary logging/assertions to narrow down the failure, then rerun `just verify`.
- Remove temporary debugging code once no longer needed.

#### Fix Failed Test Cases

- Apply fixings to source files.
- DO NOT delete any test case.
- For every attempt of fixing the source code:
  - Explain why the original change did not work.
  - Explain what you need to do.
  - Explain why you think it would solve the build break or test break.
  - Log these in `agent_execution.md`, with section `## Fixing attempt No.<attempt_number>` in `# FIXING ATTEMPTS`.
- After finishing fixing, exit the current sub agent and tell the main agent to go back to `Step 2. Compile`
  - `Step 2. Compile` and `Step 3. Run Unit Test` are absolutely no problem. If you didn't see any progress, the only reason is that your change is not correct.

## Step 4. Check it Again

- Go back to `Step 2. Compile`, follow all instructions and all steps again.

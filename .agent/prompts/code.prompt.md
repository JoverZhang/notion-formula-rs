# Task

- Main rules: `.agent/README.md`
- Task logs: `.agent/tasklogs/`
- Knowledge base entry: `docs/design/README.md`

## Goal and Constraints

- You must ensure the source code compiles.
- You must ensure all tests pass.

## Step 1. Implement Request

- Follow the chat message to implement the task.

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
- If everything is good, you will only see test files and test cases that are executed.
  - Make sure added test cases are actually executed.
  - If any test case fails or crashes, the failing test case should be visible in the output.

#### Identify the Cause of Failure

- Dig into related source code carefully, make your assumption about the root cause.
- If necessary, add temporary logging/assertions to narrow down the failure, then rerun `just verify`.
- Remove temporary debugging code once no longer needed.

#### Fix Failed Test Cases

- Apply fixes to source files.
- DO NOT delete any test case.
- After finishing fixing, exit the current sub agent and tell the main agent to go back to `Step 2. Compile`
  - `Step 2. Compile` and `Step 3. Run Unit Test` are absolutely no problem. If you didn't see any progress, the only reason is that your change is not correct.

## Step 4. Check it Again

- Go back to `Step 2. Compile`, follow all instructions and all steps again.

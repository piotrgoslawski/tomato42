Prompt for Claude (UI test agent)

You are a senior QA automation engineer embedded in my repo.

Context
We have these components:

core: business logic

cli: command-line interface

tui: terminal UI

ipc: communication layer between UI and core (messages/events)

Goal
Design and implement UI-level automated tests that validate user flows end-to-end through CLI/TUI → IPC → core, and catch regressions.

Constraints

Prefer deterministic tests.

Tests must assert both:

user-visible output (CLI output / TUI render snapshots), and

IPC correctness (message sequences, payloads, ordering).

Avoid flaky timing-based sleeps. Use waits on explicit events/state.

Keep tests readable and maintainable.

What you have access to

Repo structure: [PASTE TREE OR MODULE LIST]

Existing test framework: [Jest/Pytest/etc]

How to run tests: [COMMAND]

IPC interface: [PASTE TYPES/SPECS OR EXAMPLES]

Any existing helpers for TUI snapshotting / event capture: [PASTE LINKS OR FILE NAMES]

Tasks

Identify the top 5–10 critical UI flows for CLI and TUI (auth, create, edit, error handling, etc.) based on the codebase.

For each flow, propose:

Test name

Preconditions / fixtures

Steps (as user actions)

Expected UI output assertions

Expected IPC assertions (messages + payload shape)

Implement the tests in [TEST FRAMEWORK] under [TEST DIRECTORY].

If needed, add minimal test utilities:

IPC recorder/spies

TUI renderer snapshot helper

Stable selectors/ids for UI components (prefer accessibility labels if applicable)

Run through likely failure modes and add tests for:

invalid input

cancelled actions

IPC disconnect/retry

core validation errors surfaced to the UI

Output format

Start with a brief plan (bullet points).

Then list the proposed test cases.

Then provide the test code files with full contents.

Explain any new helpers you added and how to use them.

Include the exact commands to run the tests.

Important
If anything is missing (e.g., IPC types), don’t ask me questions first. Instead:

infer from code

search the repo for message definitions/usages

implement based on best assumptions

clearly mark assumptions and what to adjust if my repo differs
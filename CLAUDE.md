# Superpowers Integration

This project uses [Superpowers](https://github.com/obra/superpowers) — an agentic skills framework & software development methodology.

## Skills

Skills are located in `.claude/skills/`. The system auto-triggers relevant skills based on the task:

- **brainstorming** — Refines ideas before writing code
- **writing-plans** — Creates detailed implementation plans
- **test-driven-development** — RED-GREEN-REFACTOR cycle
- **systematic-debugging** — Root cause investigation
- **subagent-driven-development** — Parallel subagent execution
- **using-git-worktrees** — Isolated workspace management
- **dispatching-parallel-agents** — Concurrent agent workflows
- **executing-plans** — Batch execution with checkpoints
- **requesting-code-review** — Pre-merge code review
- **receiving-code-review** — Responding to feedback
- **verification-before-completion** — Verify before declaring done
- **finishing-a-development-branch** — Merge/PR decision workflow
- **writing-skills** — Create new skills
- **using-superpowers** — Introduction to the skills system

## Project Context

- **open-entities-lib/** — Rust library core
- **wasm-bindings/** — WASM bindings for web
- Built with Rust, WASM, and TypeScript
# tick

Task tracker with automatic daily standup report generation for team chats.

## What it does

You manage tasks and subtasks in `tick`. At the end of the day, you run one command and get a formatted text block ready to paste into your project chat:

```
Previously:
 - 🔄 Milestone name #1
 - - ✅ task name #1
 - - - ✅ subtask name #1
 - 🔄 Milestone name #2
 - - ❌ task name #1

Today:
 - 🔄 Milestone name #1
 - 🔄 Milestone name #2
 - - ✅ task name #1
 - 🛑 Milestone name #3
```

## Status Icons

| Icon | Meaning     |
|------|-------------|
| 🔄   | In progress |
| ✅    | Done        |
| ❌    | Not started |
| 🛑   | Blocked     |

## Roadmap

| Version | Description                         | Status |
|---------|-------------------------------------|--------|
| v0.1    | Multi-project support               | ✅      |
| v0.2    | CLI + SQLite                        | 🔄     |
| v0.3    | TUI frontend                        | ⏳      |
| v0.4    | React frontend with Kanban view     | ⏳      |
| v0.5    | PostgreSQL, multi-user, roles, auth | ⏳      |

## Usage (v0.1 - Projects)

Multi-project support with create, rename, switch, and scoped commands.

## Usage (v0.2 - CLI)

```bash
# Projects
tick pr ad work --title "Work"  # create project
tick pr sw work                 # switch active project
tick pr ls                      # list all projects

# Tasks
tick ts ad "Fix login bug"                 # add root task
tick ts ad "Fix login bug" -u <parent-id>  # add child task
tick ts ls                                 # list active tasks
tick ts dn <id>                            # mark done
tick ts bl <id>                            # mark blocked
tick ts rm <id>                            # delete task

# Report
tick rp                        # print today's report
tick rp -c                     # copy to clipboard
tick rp --date 2026-03-15      # report for specific date
```

## Setup

```bash
cp .env.example .env
make run
```

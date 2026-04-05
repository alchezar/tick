# tt

Task tracker with automatic daily standup report generation for team chats.

## What it does

You manage tasks and subtasks in `tt`. At the end of the day, you run one command and get a formatted text block ready to paste into your project chat:

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
| 🚫   | Abandoned   |

## Roadmap

| Version | Description                         | Status |
|---------|-------------------------------------|--------|
| v0.1    | Multi-project support               | ✅      |
| v0.2    | CLI + SQLite                        | ✅      |
| v0.3    | TUI frontend                        | ⏳      |
| v0.4    | React frontend with Kanban view     | ⏳      |
| v0.5    | PostgreSQL, multi-user, roles, auth | ⏳      |

## Usage (v0.1 - Projects)

Multi-project support with create, rename, switch, and scoped commands.

## Usage (v0.2 - CLI)

```bash
# Projects
tt pr ad work -t "Work"                  # create project
tt pr ad work -g https://github.com/o/r  # create with GitHub URL
tt pr gh work https://github.com/o/r     # set GitHub URL on existing project
tt pr sw work                            # switch active project
tt pr ls                                 # list all projects

# Tasks
tt ts ad "Fix login bug"                 # add root task
tt ts ad "Fix login bug" -p <parent-id>  # add child task
tt ts ad "Fix login bug" -d 2026-01-15   # add task with specific date
tt ts ad "Fix login bug" -n 66           # add task with PR number
tt ts ln <id> 66                         # set PR number on existing task
tt ts                                    # list active tasks
tt ts dn <id>                            # mark done
tt ts bl <id>                            # mark blocked
tt ts rm <id>                            # delete task

# Report
tt rp                # report for active project
tt rp -a             # report for all projects
tt rp -c             # copy to clipboard (without Current section)
tt rp -d 2026-03-15  # report for specific date
```

## Setup

```bash
cp .env.example .env
make run
```

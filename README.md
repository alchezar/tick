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
| v0.1    | CLI + SQLite, single project        | 🔄     |
| v0.2    | Multi-project support               | ⏳      |
| v0.3    | TUI frontend                        | ⏳      |
| v0.4    | React frontend with Kanban view     | ⏳      |
| v0.5    | PostgreSQL, multi-user, roles, auth | ⏳      |

## Usage (v0.1 CLI)

```bash
# Tasks
tick -t -a "Fix login bug"
tick -t -a "Fix login bug" -u <parent-id>   # child task
tick -t -l
tick -t -d <id>
tick -t -b <id>
tick -t --remove <id>

# Report
tick -r                        # print today's report
tick -r -c                     # copy to clipboard
```

## Usage (v0.2 — Projects)

```bash
# Manage projects
tick -p                        # show active project
tick -p -l                     # list all projects (slug + display name)
tick -p -a work                # create project with slug "work"
tick -p -a work --name "Work"  # create with display name
tick -p work                   # switch active project to "work"

# Scope any command to a project
tick -p work -t -l             # tasks in "work"
tick -p work -r -c             # report for "work", copy to clipboard
```

## Setup

```bash
cp .env.example .env
make run
```

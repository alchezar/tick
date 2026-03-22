CREATE INDEX IF NOT EXISTS idx_tasks_project_id ON tasks (project_id);
CREATE INDEX IF NOT EXISTS idx_tasks_parent_id ON tasks (parent_id);
CREATE INDEX IF NOT EXISTS idx_status_changes_task_id ON status_changes (task_id);
CREATE INDEX IF NOT EXISTS idx_status_changes_changed_at ON status_changes (changed_at);

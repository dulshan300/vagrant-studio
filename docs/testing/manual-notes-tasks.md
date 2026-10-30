# Manual test: project notes and tasks (Phase 7)

Run on Windows 11 with the local build (`pnpm build:win-local`).

## Tasks
1. Open a project. Expected: a **Tasks** card under the environments, "No tasks yet."
2. Add three tasks with Enter. Expected: they appear in order; the header says "3 open · 0 done".
3. Click a task's title, rename it, press Enter. Expected: renamed. Click again, press Esc. Expected: unchanged.
4. Hover a task and move it up and down. Expected: the order changes and survives leaving and reopening the project.
5. Tick a task. Expected: it moves to **Done (1)**; untick it there. Expected: back in the open list.
6. Delete a task. Expected: gone.

## Notes
7. **New note**: title "Database login", a few lines of text, **Save note**. Expected: listed with its first lines.
8. Type `<b>bold</b>` in a note. Expected: shown literally, not as bold text.
9. Click the note, change the text, Save. Expected: updated and listed first.
10. Start editing, change something, then click another note. Expected: asked whether to discard the changes.
11. Delete a note. Expected: asked to confirm; then gone.

## Persistence
12. Close and reopen the app. Expected: notes and tasks are unchanged.
13. Optional: remove a test project (metadata only). Expected: its notes and tasks go with it; files untouched.

Report per step, or "all pass".

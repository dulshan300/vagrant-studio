# Manual test: open in a code editor (Phase 7)

Run on Windows 11 with the local build (`pnpm build:win-local`).

1. **Settings** → *Code editor*. Expected: *Automatic (…)* names the first
   detected editor; the list shows each installed known editor (VS Code,
   Cursor, VSCodium, Zed, Sublime Text).
2. On a project page, click **Open in editor**. Expected: the editor opens
   the project folder.
3. On an environment card, **⋯** → **Open in editor**. Expected: the editor
   opens that environment's folder.
4. In Settings pick a different detected editor, Save, repeat step 2.
   Expected: that editor opens.
5. Choose *Other program…*, enter a folder path or a `.cmd` file, Save.
   Expected: an error under the field; nothing is saved. Enter a real `.exe`
   (e.g. `C:\Windows\notepad.exe`), Save. Expected: saved.
6. Set *Automatic* again and Save.

Report per step, or "all pass".

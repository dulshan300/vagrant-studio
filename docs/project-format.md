# Portable Project Format

`.vagrant-studio/project.json` describes a project so it can be imported on
another computer. It is Vagrant Studio metadata only: it doesn't replace the
Vagrantfile, which stays the source of truth for machines.

Writing the file is always an explicit action (project page → **Portable
file…**). The app shows the content first and never replaces an existing file
with different content without showing it and asking. Whether to commit it to
Git is up to you.

## Schema version 1

```json
{
  "schemaVersion": 1,
  "project": {
    "name": "Web Platform",
    "description": "Local multi-server development environment",
    "repositoryUrl": "https://github.com/owner/web-platform.git",
    "tags": ["web"]
  },
  "environments": [
    {
      "name": "development",
      "path": ".",
      "vagrantfile": "Vagrantfile",
      "defaultProvider": "virtualbox",
      "environmentType": "development",
      "machines": [
        { "name": "web", "box": "generic/ubuntu2404", "providerHint": "virtualbox" }
      ]
    }
  ]
}
```

| Field | Required | Rules |
|---|---|---|
| `schemaVersion` | yes | `1`. Files with a newer version are rejected with an "update Vagrant Studio" message. |
| `project.name` | yes | 1–100 characters, no control characters |
| `project.description` | no | ≤ 2000 characters |
| `project.repositoryUrl` | no | https, http, ssh, git or `user@host:path`; **embedded credentials are rejected** |
| `project.tags` | no | ≤ 20; letters, digits, `-`, `_`, `.`; stored lowercase |
| `environments[].name` | yes | 1–50 characters; letters, digits, space, `-`, `_`, `.`; unique (case-insensitive) |
| `environments[].path` | no (default `.`) | Relative to the project folder, `/`-separated. Absolute paths, drive letters and `..` are rejected. |
| `environments[].vagrantfile` | no | Relative file name, usually `Vagrantfile` |
| `environments[].defaultProvider` | no | Provider hint only |
| `environments[].environmentType` | no | `development`, `testing`, `staging` or `other` |
| `environments[].machines` | no | Informational hints; Vagrant remains authoritative |

Unknown fields are rejected, so typos aren't silently ignored.

## What is never written

- Absolute paths, user names or host-specific configuration
- Machine runtime IDs or anything from `.vagrant/`
- Secrets (tokens in repository URLs are refused at validation)

## Import behaviour

When a folder is imported, a valid `project.json` pre-fills the name,
description, tags and environment list. Folders found by scanning (depth 3,
skipping hidden folders, `node_modules`, `vendor`, `target`, `dist` and `build`)
are added as further candidates. An invalid file is reported and ignored; the
import continues with scanned values. Importing never writes to the folder.

## Future versions

A version 2 parser converts version 1 input in `PortableProject::parse`
(`crates/domain/src/portable.rs`) before validation, so older files keep
importing.

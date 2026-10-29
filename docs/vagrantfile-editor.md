# Vagrantfile editor

Phase 5. Implements ADR-0005. Each environment has an editor page (Project →
environment → **Vagrantfile**, or **Create Vagrantfile** when the folder has
none) with two views of the same text:

- **Raw**: CodeMirror 6 with Ruby highlighting, undo history, and search and
  replace (Ctrl/⌘+F). It saves exactly what was typed and works for any
  Vagrantfile.
- **Visual**: forms for a fixed subset of settings. Editing a form regenerates
  the whole file. Files outside the subset open in Raw, and Visual explains
  which line stopped it.

## Supported subset (`crates/config-model`)

The recognizer accepts a file only if **every** statement is one of the
following. Otherwise it reports the first line it can't represent. It
evaluates no Ruby.

```ruby
Vagrant.configure("2") do |config|           # "2" or 2; any block variable name
  config.vm.box = "…"
  config.vm.hostname = "…"
  config.vm.network "private_network", ip: "…"      # or type: "dhcp"
  config.vm.network "public_network", ip: "…", bridge: "…"  # each optional; type: "dhcp" without ip
  config.vm.network "forwarded_port", guest: 80, host: 8080, host_ip: "…", protocol: "udp"
  config.vm.synced_folder "host", "guest", disabled: true
  config.vm.provider "virtualbox" do |vb|           # or :virtualbox
    vb.cpus = 2
    vb.memory = 2048                                # or "2048"
  end
  config.vm.provider "vmware_desktop" do |v|
    v.vmx["numvcpus"] = "2"                         # or v.cpus = 2
    v.vmx["memsize"] = "2048"                       # or v.memory = 2048
  end
  config.vm.provision "shell", inline: "…" | <<-EOS … | <<~EOS … | path: "…",
    run: "once" | "always" | "never", privileged: true | false
  config.vm.define "name", primary: true, autostart: false do |m|
    # the same m.vm.* settings (no nested define, no `config.` inside)
  end
end
```

Everything else is refused, including:

- other `config.*` namespaces (`config.ssh`, `config.vagrant`, …)
- other `vm.*` settings (`box_version`, `box_check_update`, …)
- other provider options (`vb.gui`, `vb.customize`, …) and other providers
- public network options other than `ip:` and `bridge:` (an array of bridges,
  `use_dhcp_assigned_default_route:`, …)
- other provisioners and options (`auto_correct:`, `id:`, …)
- variables, conditionals, loops, `require`
- string interpolation, numeric escapes
- `<<~` heredocs with tab indentation or backslashes
- a byte order mark, and mixed line endings

A setting given twice is refused rather than resolved, as is a second
provider block for the same provider.

The model stores **values as Ruby evaluates them**. For example, an inline
script is the exact string Vagrant receives, so `<<-` keeps indentation and
`<<~` strips it. Two differently formatted files with the same meaning give
the same model.

## Network settings (Phase 6)

- **Public networks** are bridged to the host's LAN, with DHCP or a static IP.
  `bridge:` names the host adapter, and the form suggests the adapters from
  `VBoxManage list bridgedifs` that Vagrant would consider (status not `Down`
  or `Unknown`). Without `bridge:`, VirtualBox picks the only active adapter,
  but **asks interactively when there are several**. Vagrant Studio runs
  Vagrant without a terminal, so `up` then fails (`UIExpectsTTY`), and the
  form warns about an empty adapter. The VMware Desktop plugin (3.0.5)
  ignores `bridge:` and never asks. (Read in Vagrant 2.4.9's
  `plugins/providers/virtualbox/action/network.rb` and the plugin's
  `action/network.rb`.)
- **Host-port check**: while the visual editor is open, every forwarded TCP
  host port is tested the way Vagrant tests it before `up` (a TCP connect to
  the host IP, or to loopback for all interfaces; UDP isn't checked). A port
  that answers gets a warning: unless it is the environment's own running
  machine, `up` fails with a port collision. Within one Vagrantfile, a host
  port forwarded twice is already a validation error. Ports declared by
  *other* environments are a Phase 7 item (cross-project port warnings).

## Generated files

- The header holds `# vagrant-studio: sha256=<hex>`, the checksum of
  everything after that line (with LF line endings). A file with the header
  and a matching checksum is *managed*. If the checksum doesn't match, it was
  edited by hand since, and it still opens visually as long as it stays in the
  subset.
- Line endings follow the file being edited (CRLF stays CRLF).
- Scripts are written so that Ruby's value equals the model. An unquoted
  heredoc is used when the script has no `\` and no `#{`/`#@`/`#$`, otherwise
  a quoted one. `<<~` is used when a line starts at column 0, otherwise `<<-`.
  A quoted string is used when the script doesn't end with a newline.
- `generate` validates the model, then reads its own output back with the
  recognizer. It refuses to return text that doesn't give back the same
  model.
- Comments in a file that is saved from the visual editor are lost. The save
  dialog lists them by line number.

## Saving (`project-service`, `vagrantfile.rs`)

1. The command takes the environment's write lock from the operation engine,
   and refuses if any Vagrant operation for the environment is queued or
   running. Operations requested during the save wait for it.
2. The file's SHA-256 must equal the revision the editor loaded. Otherwise
   the save fails with `stale`, and the page offers *Reload* or *Keep my
   changes and compare again*.
3. If generated (managed) content would replace a file the visual editor
   didn't write, the request must set `replaceUnmanaged`. The save dialog
   asks for that with a checkbox below the diff.
4. A backup of the current file is written to
   `<app data>/backups/<environment id>/Vagrantfile.<UTC timestamp>`, and
   the newest 50 are kept. Backups never go into the project folder.
5. The new text is written to `.<name>.vagrant-studio-tmp` next to the file
   and synced, then renamed over it. On Unix it keeps the file's permissions.
   Symlinked Vagrantfiles, non-UTF-8 files and files over 1 MB are refused.
6. If the environment had no Vagrantfile, its `vagrantfilePath` is recorded.

Every save, raw or visual, shows a diff first (`@codemirror/merge`) and can
run `vagrant validate` afterwards. The **Backups** dialog loads any backup
into the raw editor, and saving restores it through the same path.

Unsaved changes are guarded on route changes and when the window closes
(`core:window:allow-destroy` lets the page close the window after the user
confirms).

## Verification

- `cargo test -p vagrant-studio-config-model`: lexer, recognizer, generator,
  validation, CRLF, tricky scripts.
- `project-service` tests:
  - every template × provider opens visually and regenerates the same model
  - stale check, replace confirmation, backups and pruning, atomic write,
    create when missing, symlink refusal
- `render_all` (opt-in) also writes each template's visual regeneration and a
  sample using every feature. All 14 files pass `vagrant validate` with
  Windows Vagrant 2.4.9. Evaluating them with Vagrant's embedded Ruby gives
  inline scripts equal to the model.

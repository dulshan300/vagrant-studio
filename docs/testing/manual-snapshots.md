# Manual walkthrough: snapshots and export

Run on Windows with a VirtualBox or VMware Desktop environment. Use a
throwaway machine: restoring discards its state.

## Snapshots
1. Start a machine and open a project's environment card. The camera button
   appears on machines that exist, and not on Docker machines.
2. Save a snapshot named `base`. It runs as an operation, and the name appears
   in the list when it finishes.
3. Save `after setup` (a name with a space works). Save `base` again: the
   operation fails with Vagrant's "--force" message and the first one stays.
4. Create a file on the machine over SSH, then press Restore on `base`. The
   action stays disabled until you type `base`.
5. After it finishes, the machine runs again, the file is gone, and the machine
   state on the card is current. Provisioners did not run.
6. Delete `after setup`, typing its name. It leaves the list.

## Export
1. Stop a machine. The export button appears on stopped machines only.
2. Choose a location and name. The output drawer opens and the `.box` file
   appears when it finishes; the machine is still there.
3. Export again to the same name: refused, the file is left alone.
4. Add the file on the Boxes page with "File" to check that it is a usable box.

## Recovery hints
1. Forward a host port that is in use and start the machine. The failed
   operation's drawer shows "What to try" under the error.
2. Quit the app during a long `up`, then start it again. The interrupted
   operation says to refresh status.

# global_bin_read_errors

Reject unreadable binary targets before activating an installation, preserving existing commands on failed updates.

## `node verify-read-errors.mjs`

```
Empty and directory bin targets fail installation without registering commands or leaving installations.
Failed updates preserve the previous installation, metadata, and working command.
```

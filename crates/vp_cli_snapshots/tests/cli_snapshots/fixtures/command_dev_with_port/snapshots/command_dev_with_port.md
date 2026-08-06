# command_dev_with_port

## `vp dev --port 12312312312`

intentionally use an invalid port (exceeds 0-65535) to trigger port error

**Exit code:** 1

```
error when starting dev server:
Error: No available ports found between 12312312312 and 65535
```

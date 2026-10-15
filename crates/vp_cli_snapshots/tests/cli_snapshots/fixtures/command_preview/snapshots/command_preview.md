# command_preview

## `vp preview --port 12312312312`

without a build, preview stops on the missing dist directory before it tries the port, as `vite preview` does

**Exit code:** 1

```
error when starting preview server:
Error: The directory "dist" does not exist. Did you build your project?
```

## `vpt mkdir dist`


## `vp preview --port 12312312312`

intentionally use an invalid port (exceeds 0-65535) to trigger port error

**Exit code:** 1

```
error when starting preview server:
Error: No available ports found between 12312312312 and 65535
```

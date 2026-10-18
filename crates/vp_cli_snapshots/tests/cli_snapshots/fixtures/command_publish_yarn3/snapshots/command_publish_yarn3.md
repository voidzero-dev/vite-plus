# command_publish_yarn3

## `vp pm publish --json -- --help`

should reject JSON before Yarn Modern supports it

**Exit code:** 1

```
yarn >= 2 && < 4.9.3 does not support --json.
```

## `vp pm publish --otp 123456 -- --help`

should reject OTP before Yarn Modern supports it

**Exit code:** 1

```
yarn >= 2 && < 3.2.0 does not support --otp.
```

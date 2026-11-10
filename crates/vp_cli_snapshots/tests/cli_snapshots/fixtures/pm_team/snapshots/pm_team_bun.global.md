# pm_team_bun

## `vpt json-edit package.json packageManager bun@1.4.0`


## `vp pm team ls @org -- --version`


## `node registry.mjs`

Bun warns and executes npm team while preserving registry failures

```
vp pm team create @org:developers --json --registry <registry> --otp <otp>
{
  "created": true,
  "team": "org:developers"
}
warn: bun does not support team, falling back to npm team.
Verified PUT /-/org/org/team with authentication and OTP.
vp pm team add @org:developers alice --json --registry <registry> --otp <otp>
{
  "added": true,
  "team": "org:developers",
  "user": "alice"
}
warn: bun does not support team, falling back to npm team.
Verified PUT /-/team/org/developers/user with authentication and OTP.
vp pm team ls @org --json --registry <registry> --otp <otp>
[
  "developers"
]
warn: bun does not support team, falling back to npm team.
Verified GET /-/org/org/team with authentication.
vp pm team list @org:developers --json --registry <registry> --otp <otp>
[
  "alice"
]
warn: bun does not support team, falling back to npm team.
Verified GET /-/team/org/developers/user with authentication.
vp pm team remove @org:developers alice --json --registry <registry> --otp <otp>
{
  "removed": true,
  "team": "org:developers",
  "user": "alice"
}
warn: bun does not support team, falling back to npm team.
Verified DELETE /-/team/org/developers/user with authentication and OTP.
vp pm team destroy @org:developers --json --registry <registry> --otp <otp>
{
  "deleted": true,
  "team": "org:developers"
}
warn: bun does not support team, falling back to npm team.
Verified DELETE /-/team/org/developers with authentication and OTP.
vp pm team create @org:developers --json --registry <registry> --otp <otp>
Registry denial returned a non-zero exit code.
Verified PUT /-/org/org/team with authentication and OTP.
```

# pm_team_pnpm

## `vpt json-edit package.json packageManager pnpm@11.13.0`


## `vp pm team ls @org -- --version`


## `node registry.mjs`

pnpm uses its native team command at the minimum supported version

```
vp pm team create @org:developers --json --registry <registry> --otp <otp>
org:developers
Verified PUT /-/org/org/team with authentication and OTP.
vp pm team add @org:developers alice --json --registry <registry> --otp <otp>
alice added to @org:developers
Verified PUT /-/team/org/developers/user with authentication and OTP.
vp pm team ls @org --json --registry <registry> --otp <otp>
[
  "developers"
]
Verified GET /-/org/org/team with authentication.
vp pm team list @org:developers --json --registry <registry> --otp <otp>
[
  "alice"
]
Verified GET /-/team/org/developers/user with authentication.
vp pm team remove @org:developers alice --json --registry <registry> --otp <otp>
-alice removed from @org:developers
Verified DELETE /-/team/org/developers/user with authentication and OTP.
vp pm team destroy @org:developers --json --registry <registry> --otp <otp>
-org:developers
Verified DELETE /-/team/org/developers with authentication and OTP.
vp pm team create @org:developers --json --registry <registry> --otp <otp>
Registry denial returned a non-zero exit code.
Verified PUT /-/org/org/team with authentication and OTP.
```

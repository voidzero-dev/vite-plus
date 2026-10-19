# pm_team_npm

## `vp pm team --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pm team <COMMAND>

Manage organization teams and memberships

Commands:
  create   Create a team in an organization
  destroy  Destroy a team in an organization
  add      Add a user to a team
  rm       Remove a user from a team [alias: remove]
  list     List teams in an organization or members of a team [alias: ls]

Options:
  -h, --help  Print help

Documentation: https://viteplus.dev/guide/package-management
```

## `vp pm team add --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pm team add [OPTIONS] <SCOPE:TEAM> <USER> [-- <PASS_THROUGH_ARGS>...]

Add a user to a team

Arguments:
  <SCOPE:TEAM>            Organization and team name
  <USER>                  Username
  [PASS_THROUGH_ARGS]...  Additional arguments

Options:
  --otp <OTP>       One-time password for authentication
  --json            Output in the package manager's native JSON format
  --registry <URL>  Registry URL
  -h, --help        Print help

Documentation: https://viteplus.dev/guide/package-management
```

## `vp pm team ls @org -- --version`


## `node registry.mjs`

all team operations use a loopback registry with fake credentials

```
vp pm team create @org:developers --json --registry <registry> --otp <otp>
{
  "created": true,
  "team": "org:developers"
}
Verified PUT /-/org/org/team with authentication and OTP.
vp pm team add @org:developers alice --json --registry <registry> --otp <otp>
{
  "added": true,
  "team": "org:developers",
  "user": "alice"
}
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
{
  "removed": true,
  "team": "org:developers",
  "user": "alice"
}
Verified DELETE /-/team/org/developers/user with authentication and OTP.
vp pm team destroy @org:developers --json --registry <registry> --otp <otp>
{
  "deleted": true,
  "team": "org:developers"
}
Verified DELETE /-/team/org/developers with authentication and OTP.
vp pm team create @org:developers --json --registry <registry> --otp <otp>
Registry denial returned a non-zero exit code.
Verified PUT /-/org/org/team with authentication and OTP.
```

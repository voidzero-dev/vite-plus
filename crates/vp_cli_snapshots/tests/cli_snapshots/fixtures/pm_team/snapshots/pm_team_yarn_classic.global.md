# pm_team_yarn_classic

## `vpt json-edit package.json packageManager yarn@1.22.22`


## `vp pm team ls @org -- --version`


## `node registry.mjs`

Yarn Classic uses its native team command without correcting scope or username values

```
vp pm team create @org:developers --json --registry <registry> --otp <otp>
{"type":"step","data":{"message":"Logging in","current":1,"total":3}}
{"type":"step","data":{"message":"Creating team","current":2,"total":3}}
{"type":"inspect","data":{"ok":true}}
{"type":"step","data":{"message":"Revoking token","current":3,"total":3}}
{"type":"info","data":"Not revoking login token, specified via config file."}
Verified PUT /team/@org with authentication.
vp pm team add @org:developers alice --json --registry <registry> --otp <otp>
{"type":"step","data":{"message":"Logging in","current":1,"total":3}}
{"type":"step","data":{"message":"Adding user to team","current":2,"total":3}}
{"type":"inspect","data":{"ok":true}}
{"type":"step","data":{"message":"Revoking token","current":3,"total":3}}
{"type":"info","data":"Not revoking login token, specified via config file."}
Verified PUT /team/@org/developers/user with authentication.
Yarn Classic sent an empty username. Its upstream behavior is preserved.
vp pm team ls @org --json --registry <registry> --otp <otp>
{"type":"step","data":{"message":"Logging in","current":1,"total":3}}
{"type":"step","data":{"message":"Listing teams","current":2,"total":3}}
{"type":"inspect","data":["developers"]}
Verified GET /org/@org/team with authentication.
vp pm team list @org:developers --json --registry <registry> --otp <otp>
{"type":"step","data":{"message":"Logging in","current":1,"total":3}}
{"type":"step","data":{"message":"Listing teams","current":2,"total":3}}
{"type":"inspect","data":["alice"]}
Verified GET /team/@org/developers/user with authentication.
vp pm team remove @org:developers alice --json --registry <registry> --otp <otp>
{"type":"step","data":{"message":"Logging in","current":1,"total":3}}
{"type":"step","data":{"message":"Removing user from team","current":2,"total":3}}
{"type":"inspect","data":{"ok":true}}
Verified DELETE /team/@org/developers/user with authentication.
Yarn Classic sent an empty username. Its upstream behavior is preserved.
vp pm team destroy @org:developers --json --registry <registry> --otp <otp>
{"type":"step","data":{"message":"Logging in","current":1,"total":3}}
{"type":"step","data":{"message":"Removing team","current":2,"total":3}}
{"type":"inspect","data":{"ok":true}}
{"type":"step","data":{"message":"Revoking token","current":3,"total":3}}
{"type":"info","data":"Not revoking login token, specified via config file."}
Verified DELETE /team/@org/developers with authentication.
vp pm team create @org:developers --json --registry <registry> --otp <otp>
Registry denial returned a non-zero exit code.
Verified PUT /team/@org with authentication.
```

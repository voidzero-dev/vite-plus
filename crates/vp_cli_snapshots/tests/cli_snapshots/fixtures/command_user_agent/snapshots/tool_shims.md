# tool_shims

Ordinary tool shims do not create a user agent and do not erase an inherited one.

## `node check-agent.cjs unset`

```
VP_USER_AGENT=(unset)
Node descendants preserve the marker.
```

## `pnpm --package vp-test-echo dlx --silent -c 'node check-agent.cjs unset pnpm'`

```
VP_USER_AGENT=(unset)
Node descendants preserve the marker.
npm_config_user_agent identifies pnpm.
```

## `npm exec --yes --package vp-test-echo -- node check-agent.cjs unset npm`

```
VP_USER_AGENT=(unset)
Node descendants preserve the marker.
npm_config_user_agent identifies npm.
```

## `npx --yes --package vp-test-echo node check-agent.cjs unset npm`

```
VP_USER_AGENT=(unset)
Node descendants preserve the marker.
npm_config_user_agent identifies npm.
```

## `VP_USER_AGENT=vp/0.0.0-parent node check-agent.cjs vp/0.0.0-parent`

```
VP_USER_AGENT=vp/<version>
Node descendants preserve the marker.
```

## `VP_USER_AGENT=vp/0.0.0-parent pnpm --package vp-test-echo dlx --silent -c 'node check-agent.cjs vp/0.0.0-parent pnpm'`

```
VP_USER_AGENT=vp/<version>
Node descendants preserve the marker.
npm_config_user_agent identifies pnpm.
```

## `VP_USER_AGENT=vp/0.0.0-parent npx --yes --package vp-test-echo node check-agent.cjs vp/0.0.0-parent npm`

```
VP_USER_AGENT=vp/<version>
Node descendants preserve the marker.
npm_config_user_agent identifies npm.
```

## `VP_SHIM_TOOL=node vp check-agent.cjs unset`

Windows trampoline-style shim dispatch must also leave the marker unset.

```
VP_USER_AGENT=(unset)
Node descendants preserve the marker.
```

## `VP_SHIM_TOOL=vpr vp check-agent`

```
$ node check-agent.cjs current ⊘ cache disabled
VP_USER_AGENT=vp/<version>
Node descendants preserve the marker.
```

## `VP_SHIM_TOOL=vpx vp ./check-agent.cjs current`

```
VP_USER_AGENT=vp/<version>
Node descendants preserve the marker.
```

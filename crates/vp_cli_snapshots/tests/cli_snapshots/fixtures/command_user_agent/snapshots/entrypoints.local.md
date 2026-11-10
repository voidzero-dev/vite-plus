# entrypoints

Each explicit vp, vpr, or vpx invocation replaces an inherited user agent with its own version.

## `vp exec node check-agent.cjs current`

```
VP_USER_AGENT=vp/<version>
Node descendants preserve the marker.
```

## `vp run check-agent`

```
$ node check-agent.cjs current ⊘ cache disabled
VP_USER_AGENT=vp/<version>
Node descendants preserve the marker.
```

## `vpr check-agent`

```
$ node check-agent.cjs current ⊘ cache disabled
VP_USER_AGENT=vp/<version>
Node descendants preserve the marker.
```

## `vpx ./check-agent.cjs current`

```
VP_USER_AGENT=vp/<version>
Node descendants preserve the marker.
```

## `vp dlx -s -p vp-test-echo -c 'node check-agent.cjs current pnpm'`

The package manager replaces its own user agent but preserves VP_USER_AGENT.

```
VP_USER_AGENT=vp/<version>
Node descendants preserve the marker.
npm_config_user_agent identifies pnpm.
```

## `vpx -s -p vp-test-echo -c 'node check-agent.cjs current pnpm'`

```
VP_USER_AGENT=vp/<version>
Node descendants preserve the marker.
npm_config_user_agent identifies pnpm.
```

## `vpt rm package.json`


## `vp dlx -s -p vp-test-echo -c 'node check-agent.cjs current pnpm'`

The marker also reaches dlx children outside a project.

```
VP_USER_AGENT=vp/<version>
Node descendants preserve the marker.
npm_config_user_agent identifies pnpm.
```

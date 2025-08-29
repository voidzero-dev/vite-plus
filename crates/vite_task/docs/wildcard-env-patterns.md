# Common Wildcard Pattern for Task Envs

## Executive Summary

This design document outlines the implementation of wildcard pattern support for environment variable matching in vite-plus task configurations. This feature allows users to specify environment variables using glob-like patterns (e.g., `NODE_*`, `VITE_*`, `MY_APP_*`, `*_PORT`) instead of listing each variable explicitly.

## Background

Currently, vite-plus requires explicit listing of environment variables in task configurations:

```json
{
  "tasks": {
    "build": {
      "command": "vite build",
      "envs": ["NODE_ENV", "NODE_OPTIONS", "VITE_API_URL", "VITE_APP_TITLE", "MY_APP_PORT"]
    }
  }
}
```

This approach becomes cumbersome when dealing with multiple environment variables that follow a naming pattern, especially for frameworks like Vite that use prefixed variables (`VITE_*`).

## Goals

1. **Simplify Configuration**: Allow wildcard patterns in the `envs` array to match multiple environment variables
2. **Maintain Cache Correctness**: Ensure wildcard-matched variables are properly included in cache fingerprints
3. **Backward Compatibility**: Support both explicit variable names and wildcard patterns
4. **Performance**: Minimal overhead when resolving environment variables
5. **Security**: Automatically mask sensitive environment variables in console output

## Non-Goals

1. Full regex support (only glob-style wildcards)
2. Wildcard patterns in `pass_through_envs` (same as `envs`)
3. Complex glob patterns like `{VITE,NODE}_*` (supported by wax crate)

## Proposed Solution

### Pattern Syntax

Support standard glob wildcard patterns:

- `*` - Matches zero or more characters
- `?` - Matches exactly one character

Examples:

- `NODE_*` - Matches `NODE_ENV`, `NODE_OPTIONS`, etc.
- `VITE_*` - Matches all Vite environment variables
- `*_PORT` - Matches `API_PORT`, `SERVER_PORT`, etc.
- `REACT_APP_*` - Matches Create React App variables
- `APP?_*` - Matches `APP1_NAME`, `APP2_NAME`, etc., but not `APP_NAME`

#### No Support for Negated Patterns

We don't support `!` for negated patterns. If match the negated pattern, will ignore it and show a warning.

> Explicitness over Convenience

### Implementation Architecture

```
┌─────────────────┐
│  Task Config    │
│   envs: [       │
│     "NODE_*",   │
│     "VITE_*",   │
│     "CI"        │
│   ]             │
└────────┬────────┘
         │
         ▼
┌───────────────────┐
│ Pattern Matcher   │
│                   │
│ - Parse patterns  │
│ - Match env vars  │
└────────┬──────────┘
         │
         ▼
┌─────────────────┐
│ Resolved Envs   │
│                 │
│ NODE_ENV        │
│ NODE_OPTIONS    │
│ VITE_API_URL    │
│ VITE_BASE_URL   │
│ CI              │
└────────┬────────┘
         │
         ▼
┌───────────────────┐
│ Cache Fingerprint │
│                   │
│ Stable, sorted    │
│ env list          │
└───────────────────┘
```

#### Cache Fingerprint Stability and Security

The implementation must ensure:

1. Environment variables are sorted alphabetically before inclusion in fingerprint
2. The same wildcard pattern always produces the same set of variables (for the same environment)
3. Cache keys remain stable across runs
4. **Sensitive values are hashed, never stored in plaintext**

**Security-First Fingerprint Implementation:**

```rust
use sha2::{Sha256, Digest};

impl TaskEnvs {
    /// Create a secure fingerprint for environment variables
    pub fn create_fingerprint(&self) -> HashMap<Str, Str> {
        let mut fingerprint = HashMap::new();
        
        for (name, value) in &self.envs_without_pass_through {
            let fingerprint_value = if is_sensitive_env_name(name) {
                // Hash sensitive values - never store them in plaintext
                let mut hasher = Sha256::new();
                hasher.update(value.as_bytes());
                format!("sha256:{:x}", hasher.finalize())
            } else {
                // Non-sensitive values can optionally be hashed too for consistency
                // Or stored as-is if needed for debugging
                value.clone()
            };
            
            fingerprint.insert(name.clone(), fingerprint_value);
        }
        
        fingerprint
    }
}

// In CommandFingerprint
#[derive(Encode, Decode, Debug, Serialize, PartialEq, Eq, Diff, Clone)]
pub struct CommandFingerprint {
    pub cwd: Str,
    pub command: TaskCommand,
    /// Environment variable fingerprints (names + hashed values for sensitive vars)
    /// NEVER contains actual sensitive values
    pub envs_fingerprint: HashMap<Str, Str>,
}
```

**Key Security Principles:**

- Sensitive environment values are ALWAYS hashed before storage
- Use cryptographic hash (SHA-256) for one-way transformation
- Hash includes a prefix (e.g., "sha256:") to identify the method
- Cache entries never contain plaintext secrets
- Fingerprint comparison still works (same value = same hash)

### Configuration Examples

#### Before (Current)

```json
{
  "tasks": {
    "build": {
      "command": "vite build",
      "envs": [
        "NODE_ENV",
        "NODE_OPTIONS",
        "VITE_API_URL",
        "VITE_APP_TITLE",
        "VITE_PUBLIC_PATH",
        "VITE_BASE_URL"
      ]
    }
  }
}
```

#### After (With Wildcards)

```json
{
  "tasks": {
    "build": {
      "command": "vite build",
      "envs": ["NODE_*", "VITE_*"]
    }
  }
}
```

### Testing Strategy

- Test with real Vite projects using `VITE_*` variables
- Test with Node.js projects using `NODE_*` variables
- Test cache hit/miss scenarios with wildcard patterns

### Performance Considerations

1. **Pattern Compilation**: Compile wildcard patterns once during task configuration parsing
2. **Caching**: Cache resolved environment variable lists per task to avoid repeated pattern matching
3. **Lazy Evaluation**: Only resolve wildcards when actually needed for task execution

### Security Considerations

#### Sensitive Environment Variable Masking

When displaying environment variables in logs or console output, sensitive values must be automatically masked with `***` to prevent accidental exposure of secrets.

**Known Sensitive Patterns:**

| Pattern         | Description                 | Example Variables                            |
| --------------- | --------------------------- | -------------------------------------------- |
| `*_KEY`         | API keys, encryption keys   | `API_KEY`, `SECRET_KEY`, `PRIVATE_KEY`       |
| `*_SECRET`      | Secrets and sensitive data  | `APP_SECRET`, `JWT_SECRET`                   |
| `*_TOKEN`       | Authentication tokens       | `AUTH_TOKEN`, `ACCESS_TOKEN`, `GITHUB_TOKEN` |
| `*_PASSWORD`    | Passwords                   | `DB_PASSWORD`, `ADMIN_PASSWORD`              |
| `*_PASS`        | Short form passwords        | `MYSQL_PASS`, `REDIS_PASS`                   |
| `*_PWD`         | Alternative password form   | `DB_PWD`, `USER_PWD`                         |
| `*_CREDENTIAL*` | Credentials                 | `AWS_CREDENTIALS`, `CREDENTIAL_ID`           |
| `*_API_KEY`     | API keys                    | `STRIPE_API_KEY`, `GOOGLE_API_KEY`           |
| `*_PRIVATE_*`   | Private data                | `PRIVATE_KEY`, `PRIVATE_TOKEN`               |
| `AWS_*`         | AWS credentials             | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` |
| `GITHUB_*`      | GitHub tokens               | `GITHUB_TOKEN`, `GITHUB_PAT`                 |
| `NPM_*TOKEN`    | NPM tokens                  | `NPM_TOKEN`, `NPM_AUTH_TOKEN`                |
| `DATABASE_URL`  | Database connection strings | Contains passwords                           |
| `MONGODB_URI`   | MongoDB connection strings  | Contains passwords                           |
| `REDIS_URL`     | Redis connection strings    | May contain passwords                        |
| `*_CERT*`       | Certificates                | `SSL_CERT`, `TLS_CERT_KEY`                   |

**Implementation Example:**

```rust
fn is_sensitive_env_name(name: &str) -> bool {
    const SENSITIVE_PATTERNS: &[&str] = &[
        "*_KEY",
        "*_SECRET",
        "*_TOKEN",
        "*_PASSWORD",
        "*_PASS",
        "*_PWD",
        "*_CREDENTIAL*",
        "*_API_KEY",
        "*_PRIVATE_*",
        "AWS_*",
        "GITHUB_*",
        "NPM_*TOKEN",
        "DATABASE_URL",
        "MONGODB_URI",
        "REDIS_URL",
        "*_CERT*",
    ];
    
    // Exact matches for known sensitive names
    const SENSITIVE_EXACT: &[&str] = &[
        "PASSWORD",
        "SECRET",
        "TOKEN",
        "PRIVATE_KEY",
        "PUBLIC_KEY",
    ];
    
    if SENSITIVE_EXACT.contains(&name) {
        return true;
    }
    
    for pattern in SENSITIVE_PATTERNS {
        if Glob::new(pattern).is_match(name) {
            return true;
        }
    }
    
    false
}

fn display_env_value(name: &str, value: &str) -> String {
    if is_sensitive_env_name(name) {
        format!("{}=***", name)
    } else {
        format!("{}={}", name, value)
    }
}
```

#### Cache Storage Security

**Principles for Secure Cache Storage:**

1. **Never Store Sensitive Values in Cache**:
   ```rust
   // BAD - Never do this
   cache_entry.envs = task.envs_without_pass_through.clone();

   // GOOD - Store hashed fingerprints
   cache_entry.env_fingerprint = task.create_secure_fingerprint();
   ```

2. **Use One-Way Hashing**:
   - SHA-256 for sensitive values (irreversible)
   - Consistent hashing ensures cache hits work correctly
   - Different secrets produce different hashes

3. **Cache File Protection**:
   - Store cache files with restricted permissions (0600)
   - Consider encrypting the entire cache database
   - Regular cache cleanup to minimize exposure window

4. **Example Cache Entry Structure**:
   ```json
   {
     "task_id": "build_abc123",
     "env_fingerprint": {
       "NODE_ENV": "production",
       "API_KEY": "sha256:a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3",
       "DATABASE_URL": "sha256:2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae"
     },
     "outputs": "...",
     "timestamp": "2024-01-01T00:00:00Z"
   }
   ```

#### Additional Security Measures

1. **Secret Leakage Prevention**:
   - Wildcard patterns could inadvertently capture sensitive environment variables
   - Mitigation: Warn users when using broad patterns like `*` or `*_*`
   - Provide clear documentation about security implications

2. **Cache Poisoning**:
   - Malicious environment variables matching wildcards could affect cache
   - Mitigation: Validate environment variable names and values
   - Use hashed values in cache keys to prevent manipulation

3. **Audit Logging**:
   - Log when sensitive patterns are detected (without values)
   - Track which tasks access sensitive environment variables
   - Provide security audit trails for compliance

4. **Runtime Protection**:
   - Clear sensitive values from memory after use
   - Use secure string types that zero memory on drop
   - Implement rate limiting for cache operations

## Open Questions

1. Should we support `?` for single character matching? (yes)
2. Should we warn when wildcards match > 100 variables? (no)
3. Should we support exclusion patterns (e.g., `!SECRET_*`)? (no)

## References

- [Turborepo Environment Variable Handling](https://turborepo.com/docs/crafting-your-repository/caching#environment-variables)
- [Vite Environment Variables](https://vite.dev/guide/env-and-mode.html)
- [wax Crate Documentation](https://docs.rs/wax/)

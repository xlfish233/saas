# Technical Debt Register

> This document tracks technical decisions that may need future attention.

---

## Active Items

### 1. Runtime SQLx Queries (sqlx-shim)

**Status**: Active
**Added**: 2026-03-03
**Priority**: Medium

#### Context

We created `crates/sqlx-shim` to enable runtime SQL queries instead of using SQLx's compile-time macro verification. This decision trades compile-time safety for development convenience.

#### Rationale

| Benefit | Cost |
|---------|------|
| No database connection needed for compilation | No compile-time SQL verification |
| Faster compile times (no macro expansion) | SQL errors only detected at runtime |
| Simpler CI (no SQLX_OFFLINE required) | Manual `FromRow` implementations required |
| No .sqlx cache files to maintain | Field name typos caught at runtime |

#### Triggers for Re-evaluation

- [ ] Before production deployment
- [ ] If SQL-related bugs increase in frequency
- [ ] When onboarding new developers (training cost)
- [ ] If performance becomes critical (query optimization)

#### Potential Solutions

1. **Hybrid Approach**: Use compile-time macros for critical paths (auth, payments) while keeping runtime queries for less critical code.

2. **Code Generation**: Create a build.rs script that generates FromRow implementations from database schema.

3. **Full Migration**: Return to standard SQLx with offline mode when the schema stabilizes.

#### References

- Commit: `5b2b521` - "refactor: add sqlx-shim for runtime queries"
- File: `crates/sqlx-shim/src/lib.rs`

---

## Resolved Items

*None yet*

---

## Template for New Items

```markdown
### N. [Title]

**Status**: Active | Monitoring | Resolved
**Added**: YYYY-MM-DD
**Priority**: High | Medium | Low

#### Context
[Why was this decision made? What is the current situation?]

#### Rationale
[What benefits does the current approach provide? What costs?]

#### Triggers for Re-evaluation
- [ ] Trigger 1
- [ ] Trigger 2

#### Potential Solutions
1. Solution A
2. Solution B

#### References
- Related commits, files, or discussions
```

---

*Last updated: 2026-03-03*

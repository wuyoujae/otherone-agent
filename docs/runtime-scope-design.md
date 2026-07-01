# Runtime Scope Design

## Goal

Make Otherone reusable for authenticated, multi-tenant, multi-project, and future context-aware Agent
applications without baking application-specific fields such as `user_id` directly into the core
agent loop.

The framework should understand a generic runtime context. Applications decide which business
attributes are required.

Examples:

- user-scoped personal agent: `user_id`
- team workspace agent: `tenant_id`, `workspace_id`, `user_id`
- project agent: `app_id`, `project_id`, `environment`
- hosted SaaS agent: `tenant_id`, `region`, `user_id`

## Current Problem

Current storage and context APIs are keyed only by `session_id` and optional `database_config`.

Affected public types:

- `otherone_agent::types::InputOptions`
- `otherone_context::types::CombineContextOptions`
- `otherone_storage::types::WriteEntryOptions`
- `otherone_storage::types::WriteCompactedEntryOptions`

Current SQL tables also only use `session_id`:

- `otherone_session`
- `otherone_entries`
- `otherone_compacted_entries`

This is fine for single-user local usage, but unsafe for shared SQL storage because different
applications/users/tenants can collide or read each other's sessions.

## Design Decision

Introduce a generic `RuntimeContext` with arbitrary application attributes.

Do not add only `user_id`.
Do not make a fixed set of business fields part of the framework schema.

The framework needs one stable storage isolation key plus an extensible attribute bag:

```rust
pub struct RuntimeContext {
    pub partition_key: String,
    pub attributes: AttributeBag,
}

pub type AttributeBag = BTreeMap<String, serde_json::Value>;
```

Meaning:

- `partition_key` is the mandatory storage and authorization boundary for shared backends.
- `attributes` contains application-defined business fields.
- Applications can add, remove, rename, or change business fields without changing the agent loop.
- The framework validates attribute names and values, but does not interpret their business meaning.

Examples:

```rust
// Personal agent
RuntimeContext {
    partition_key: "user:usr_123".to_string(),
    attributes: {
        "user_id": "usr_123",
        "account_plan": "pro"
    }
}

// Team workspace agent
RuntimeContext {
    partition_key: "tenant:t_1:workspace:w_9".to_string(),
    attributes: {
        "tenant_id": "t_1",
        "workspace_id": "w_9",
        "user_id": "usr_123",
        "role": "admin",
        "region": "ap-east-1"
    }
}
```

`RuntimeContext` is part of the framework contract and travels through:

```text
caller -> InputOptions -> agent loop -> context loader -> storage writer/reader -> tools/memory
```

Rules:

- `partition_key` is required for SQL/shared storage.
- `attributes` stores application fields such as `user_id`, `tenant_id`, `workspace_id`,
  `project_id`, `app_id`, `environment`, `role`, or future custom values.
- Keys are ASCII snake_case.
- Values must be JSON primitives or small arrays/objects.
- Attributes must not contain secrets, raw tokens, passwords, or large payloads.
- Runtime context is server trusted. It must come from the application backend, not directly from
  the client.
- The framework validates required context rules before using SQL/shared storage.

## Why `partition_key` Plus Arbitrary Attributes

Business fields change. Storage isolation should not.

`partition_key` gives the framework a stable, compact leading boundary for all storage queries.
`attributes` gives applications unlimited business metadata without schema changes.

This means a future app can add fields such as:

- `organization_id`
- `team_id`
- `workspace_id`
- `project_id`
- `repository_id`
- `billing_account_id`
- `data_region`
- `deployment_environment`
- `permission_version`

without changing Otherone core types or SQL columns.

Important distinction:

- Fields that define data ownership must be included in `partition_key` or policy checks.
- Fields that are only business metadata can live only in `attributes`.

The framework cannot guess which arbitrary field is a security boundary. The application must decide
how to build `partition_key` and which attributes are required.

## Public API Shape

Add runtime context to top-level agent input:

```rust
pub struct InputOptions {
    pub session_id: String,
    pub runtime_context: Option<RuntimeContext>,
    ...
}
```

Add runtime context to context options:

```rust
pub struct CombineContextOptions {
    pub session_id: String,
    pub runtime_context: Option<RuntimeContext>,
    ...
}
```

Add runtime context to storage write/read/list options:

```rust
pub struct WriteEntryOptions {
    pub storage_type: StorageType,
    pub session_id: String,
    pub runtime_context: Option<RuntimeContext>,
    pub metadata: AttributeBag,
    ...
}

pub struct ReadSessionOptions {
    pub storage_type: StorageType,
    pub session_id: String,
    pub runtime_context: Option<RuntimeContext>,
    pub database_config: Option<DatabaseConfig>,
}

pub struct ListSessionsOptions {
    pub storage_type: StorageType,
    pub runtime_context: Option<RuntimeContext>,
    pub database_config: Option<DatabaseConfig>,
    pub filters: Vec<AttributeFilter>,
}
```

Keep old functions as compatibility wrappers where reasonable:

```rust
read_session_data(session_id)
read_session_data_from_database(session_id, config)
```

But add context-aware APIs and mark unscoped SQL helpers as unsafe for shared multi-tenant use in
docs:

```rust
read_session_data_with_context(options)
get_sessions_with_context(options)
write_entry(options)
```

## Storage Semantics

LocalFile:

- Runtime context is optional for backward compatibility.
- Existing apps may continue using `Otherone::set_localfile_root`.
- Multi-user apps should set a per-context localfile root.
- Optional future enhancement: localfile can derive a subdirectory from `partition_key`.

SQL:

- Runtime context is required.
- Missing `partition_key` returns `StorageError::ConfigError`.
- Every read/write/list/update/delete query filters by `partition_key`.
- `session_id` uniqueness is scoped by `partition_key`.
- `attributes` and record `metadata` are stored as JSON.
- If an application needs to query arbitrary attributes efficiently, it can declare indexed
  attributes without changing the main schema.

MongoDB:

- Runtime context is required for shared collections.
- Documents store `partition_key`, `attributes`, and `metadata`.
- Queries include `partition_key`.

Redis:

- `partition_key` is part of cache key prefixes.
- Redis remains a cache layer, not source of truth.

## SQL Schema

Use `partition_key` as the stable isolation key.

PostgreSQL/MySQL logical schema:

```text
otherone_session
  partition_key
  session_id
  status
  create_at
  updated_at
  attributes_json
  metadata_json
  primary key(partition_key, session_id)
  index(partition_key, status, create_at)

otherone_entries
  partition_key
  entry_id
  session_id
  content
  role
  token_consumption
  status
  tools
  create_at
  is_compaction
  attributes_json
  metadata_json
  primary key(partition_key, entry_id)
  foreign key(partition_key, session_id) references otherone_session(partition_key, session_id)
  index(partition_key, session_id, create_at)

otherone_compacted_entries
  partition_key
  entry_id
  session_id
  trigger_entry_id
  summary
  create_at
  status
  attributes_json
  metadata_json
  primary key(partition_key, entry_id)
  foreign key(partition_key, session_id) references otherone_session(partition_key, session_id)
  index(partition_key, session_id, create_at)
```

Do not make `(user_id, session_id)` the framework schema. `user_id` belongs to applications.

## Arbitrary Attribute Indexing

Arbitrary fields must not require schema changes.

Store every application-defined field in JSON:

```text
attributes_json   # runtime/application context fields
metadata_json     # per-session or per-entry extra business fields
```

For efficient cross-database lookup, add a generic optional index table:

```text
otherone_attribute_index
  partition_key
  entity_type        # session | entry | compacted_entry
  entity_id
  attribute_source   # context | metadata
  attribute_key
  value_type
  value_text
  value_hash
  created_at
  index(partition_key, attribute_key, value_hash)
  index(partition_key, entity_type, attribute_key)
```

Applications choose which attributes to index:

```rust
pub struct AttributeIndexPolicy {
    pub context_keys: Vec<String>,
    pub metadata_keys: Vec<String>,
}
```

Examples:

- Index `project_id` for listing project sessions.
- Index `repository_id` for code-agent history.
- Index `environment` for staging/prod separation.
- Do not index large or sensitive values.

The source of truth remains JSON. The index table is a projection and can be rebuilt.

## Required Context Policy

Applications need a way to declare required context fields without changing framework structs.

Suggested config:

```rust
pub struct RuntimeContextPolicy {
    pub require_partition_key_for_shared_storage: bool,
    pub required_attributes: Vec<String>,
    pub indexed_attributes: AttributeIndexPolicy,
}
```

Default policy:

- localfile: runtime context optional.
- SQL/MongoDB/Redis: `partition_key` required.
- attributes are not required by the framework unless the caller sets a policy.

Application example:

```rust
let context = RuntimeContext::builder("user:usr_123")
    .attr("user_id", "usr_123")
    .attr("account_plan", "pro")
    .build()?;
```

Hosted team example:

```rust
let context = RuntimeContext::builder("tenant:t_1:workspace:w_9")
    .attr("tenant_id", "t_1")
    .attr("workspace_id", "w_9")
    .attr("user_id", "usr_123")
    .attr("role", "admin")
    .build()?;
```

## Agent Loop Changes

The agent loop should not inspect business attributes.

It should only forward `input.runtime_context` to:

- `WriteEntryOptions`
- `CombineContextOptions`
- queued user prompt writes
- assistant response writes
- tool response writes
- compacted entry writes through `otherone-context`

This keeps business isolation out of the reasoning loop.

## Context Changes

`combine_context` receives the same runtime context and passes it to storage readers.

Compaction must also pass runtime context to `write_compacted_entry`; otherwise summaries can leak
into a global SQL table.

## Tools And Memory

Future framework-level tools should receive a runtime context:

```rust
pub struct ToolRuntimeContext {
    pub runtime_context: Option<RuntimeContext>,
}
```

For now, existing `tools_realize` closures do not accept context. To preserve compatibility, add a
new optional context-aware tool interface later instead of breaking the current one.

Memory currently uses global localfile root state. Multi-context apps should either:

- set a per-context memory root before invoking the agent, or
- use a future context-aware memory API:

```rust
read_memory_tree(runtime_context)
write_memory_tree(runtime_context, tree)
```

## Backward Compatibility

Keep source compatibility as much as possible:

- Add `runtime_context: Option<RuntimeContext>` to struct literals is a breaking Rust API change.
- To reduce future breaking changes, introduce builders for public option types:
  - `InputOptionsBuilder`
  - `WriteEntryOptionsBuilder`
  - `CombineContextOptionsBuilder`

Recommended release approach:

1. Add `RuntimeContext`, `AttributeBag`, and builder APIs.
2. Keep structs public for one release but document builders as preferred.
3. In the next breaking version, consider making fields private to allow future expansion.

## Migration From Current SQL Tables

Unscoped SQL tables are not safe to auto-migrate into shared scoped tables because ownership is
unknown.

Migration options:

- Single-user database: migrate all old rows into a caller-provided `partition_key`.
- Shared database: require explicit export/import per known owner.
- New installations: create context-aware schema only.

Startup safety:

- If SQL storage detects old unscoped tables in shared mode, fail fast with a clear error.

## Implementation Steps

1. Add `RuntimeContext`, `AttributeBag`, `AttributeFilter`, and validation helpers to
   `otherone-storage::types`.
2. Add context-aware read/list option types to `otherone-storage`.
3. Update Postgres/MySQL schema creation to use `partition_key`, `attributes_json`, and
   `metadata_json`.
4. Add `otherone_attribute_index` for optional arbitrary attribute indexing.
5. Update Postgres/MySQL read/write/list queries to require and filter `partition_key`.
6. Update MongoDB queries to store/filter `partition_key`.
7. Update Redis cache keys to include `partition_key`.
8. Add `runtime_context` to `InputOptions` and `CombineContextOptions`.
9. Pass runtime context through agent loop and context compaction.
10. Add facade exports in `otherone`.
11. Add tests:
    - SQL write requires runtime context.
    - two partition keys can use the same session_id without mixing data.
    - arbitrary attributes persist without schema changes.
    - indexed attributes can filter sessions.
    - list sessions returns only one partition key.
    - compaction writes summaries into the same partition key.
    - localfile remains backward compatible.

## Risks

- Public struct field additions are breaking for callers using struct literals.
- SQL schema migration must not silently mix old unscoped data with new scoped data.
- Global localfile/memory roots remain process-global and need careful handling in concurrent apps.
- Context-aware tools require a future API addition to avoid breaking current closures.

## Rollback

- Keep old unscoped localfile behavior unchanged.
- Keep old helper functions for localfile and single-user use.
- Gate context-aware SQL schema behind new init functions until tested.
- If needed, publish a minor release with builders first, then a major release that requires scope
  for shared storage.

---
type: ArchitectureProposal
title: Addressable resources, credential references, and local realizations
description: A broad resource-record design with tiny credential URI references, filesystem persistence, SlateDB adoption, RustFS fixtures, and a proposed epic map extending toward table formats and local checkouts.
resource: /design/remote-resources/draft0.gpt6a.md
status: draft
generated: { by: "model:openai/gpt-6-astra#medium", at: 2026-10-22 }
sources:
  - resource: "opencode://ses_e9ab1e8adffeMXpHDA7AZzAeRm"
    title: Human direction for credential URIs, optional metadata, local checkout relationships, SlateDB and RustFS exploration; source checks and first design/ticket proposal
  - resource: /design/fd-handoff/architecture0.gpt6a.md
    title: Composable capability layers and independently optional dependencies
  - resource: /design/fd-handoff/systemd-native-adoption0.gpt6a.md
    title: Names-first application adoption and independent metadata carriers
  - resource: /handles/core/src/lib.rs
    title: Current descriptor-constrained handle table
  - resource: /state/core/src/lib.rs
    title: Existing versioned opaque metadata extensions
  - resource: "file:///home/rektide/archive/slatedb/slatedb/slatedb/src/db.rs"
    title: Bucket-only object-store resolver, separate database path, and writer-fencing test
  - resource: "file:///home/rektide/archive/slatedb/slatedb/bindings/uniffi/src/object_store_builder.rs"
    title: Provider-specific URL and configuration builders
  - resource: "file:///home/rektide/archive/systemd/systemd/man/systemd.exec.xml"
    title: LoadCredential, encrypted credentials, service-scoped credential directory and lifetime
  - resource: "https://www.iana.org/assignments/media-types/application.csv"
    title: Application media-type registry inspected for SlateDB, Arrow, Parquet and Iceberg
  - resource: "https://www.iana.org/assignments/media-types/application/vnd.apache.parquet"
    title: Registered Parquet file media type
  - resource: "file:///home/rektide/archive/apache/arrow/docs/source/format/Columnar.rst"
    title: Arrow IPC file and stream media types
  - resource: "https://github.com/rustfs/rustfs"
    title: RustFS S3 service, standalone/container deployment and advertised compatibility matrices
---

# Addressable resources and local realizations

**Pass a description of what can be opened, with separately resolvable credential
references. Let the same vocabulary describe an existing local realization of a
remote resource.** SlateDB is the first concrete consumer; credentials are a
reusable capability in their own right.

This is a design wave for discussion. The epic IDs below are proposed tracking
names, not newly created tickets. The human direction is deliberately broad:
declare useful optional fields and relationships now, then choose implementation
slices. The first experiment need not implement every kind it can describe.

## Constituent parts

| Part | Meaning | Example |
| --- | --- | --- |
| Resource record | Named description of an addressable thing | A SlateDB database or an S3 bucket |
| Locator | Where the thing is reached within an explicit resolution context | `s3://example/catalog` |
| Kind | Semantic interpretation and adapter selection | `slatedb`, `object-store`, `iceberg-table`, `credential` |
| Media type | Encoding of a particular representation's bytes | `application/vnd.apache.arrow.file` |
| Credential reference | URI naming where authentication material is obtained | `systemd-cred:catalog` |
| Connection profile | Non-secret endpoint/provider settings | RustFS endpoint, region, addressing style |
| Realization | A local copy, checkout, cache or materialization of something | A local directory tied to a source revision |
| Observation | What was checked, when, against which source/version | Credential readable; database reopened; copy complete |

A location is not necessarily an immutable identity. An S3 bucket name also
needs its provider/endpoint context; the same bucket name at AWS and a local
RustFS server describes different storage. Keep connection context attached to
the locator rather than relying on whichever endpoint happens to be ambient.

Resource descriptions can be copied freely without transferring an open
connection or granting access. Resolved credentials and opened databases are
runtime objects with their own lifetimes. A transferable description and a
usable handle are related states, not interchangeable proofs.

## A small record with room to grow

Proposed JSON representation, illustrative rather than a frozen schema:

```json
{
  "version": 1,
  "id": "catalog",
  "kind": "slatedb",
  "uri": "s3://example/catalog",
  "connection": {
    "endpoint": "http://127.0.0.1:9000",
    "region": "us-east-1",
    "addressing": "path",
    "allow_http": true
  },
  "creds": "systemd-cred:catalog",
  "labels": { "purpose": "catalog", "environment": "fixture" }
}
```

The adapter splits bucket URL and database prefix for SlateDB. Preserve object
key semantics: do not apply filesystem normalization to prefixes. Specify URL
escaping, trailing slashes and empty-prefix behavior in the schema ticket.

Optional fields worth supporting include `media_type`, `format_version`,
`profile`, `description`, `labels`, `extensions`, `relations`, a revision selector,
and role-specific credential bindings. Labels are arbitrary string pairs;
extensions can hold structured application data. Required semantic extensions
must be recognizable as required, so a reader does not silently open a different
resource after ignoring something essential.

The record itself may be JSON while the target is a database, directory or Arrow
file. `media_type` describes the target representation, not the JSON envelope.
Record encoding and target encoding must remain separately expressible.

### Credential URI, record, and field sugar

The universal type is a **credential reference**, with a required URI and
optional descriptive/interpretive metadata. Systemd is one resolver for it.

The tiny file `catalog-creds` can contain just:

```text
systemd-cred:catalog
```

A rich form, when needed:

```json
{
  "version": 1,
  "id": "catalog-creds",
  "kind": "credential",
  "uri": "systemd-cred:catalog",
  "media_type": "application/json",
  "profile": "aws-static-credentials-v1",
  "labels": { "purpose": "object-store-auth" }
}
```

`aws-static-credentials-v1` is a proposed local profile, not an AWS standard.
Its future definition would name fields such as access key ID, secret access
key and optional session token. JSON alone does not define those meanings.
The URI scheme chooses retrieval; the media type chooses decoding; the profile
chooses interpretation. None substitutes for the others.

Support both compact embedding and reusable records:

```json
{ "creds": "systemd-cred:catalog" }
```

```json
{ "creds": { "ref": "../credentials/catalog-creds" } }
```

The string expands in memory to `{ "uri": "systemd-cred:catalog" }`; it does
not create a file. `ref` names a record relative to the containing document's
base. An embedded rich reference can carry the same metadata. Export must retain
that base or package/rewrite references explicitly. If both a URI and `ref` are
given in one binding, reject the ambiguity rather than invent precedence.

For multiple authentication roles, grow toward a `credentials` map, for example
`storage`, `wal`, and `catalog`. Define `creds` as shorthand for the default role;
reject competing definitions of that role. This lets a later Iceberg catalog
and its object storage use different credentials.

Missing credentials mean “no credential binding declared”; the consuming profile
must specify whether anonymous or ambient provider credentials are permitted.
An explicitly declared reference that fails to resolve must not silently become
an ambient identity. A name is not globally portable: exporting it preserves
the intended binding, while a new environment supplies or remaps its context.

### Systemd resolver

Proposed private URI grammar: `systemd-cred:<credential-id>`. This is our
convention, not a claim of an existing registered systemd URI scheme.

Resolve against an explicitly supplied credential directory, with an optional
adapter capturing `$CREDENTIALS_DIRECTORY`. Accept a single valid credential
name; specify encoding and reject path traversal, separators, query/fragment
ambiguity and malformed names. Do not interpret the name as a path into the
manager's global credential stores.

```ini
[Service]
LoadCredential=catalog:/configured/source/catalog.json
```

The unit makes the credential available; the record names the binding the app
expects. Resolving the URI does not configure the unit or cause systemd to load
new credentials. `LoadCredentialEncrypted=` changes provisioning; the app still
reads plaintext bytes from its service-scoped directory. TPM binding is an
optional provisioning choice, not a property conveyed by the URI.

Separate reference validation, byte retrieval, profile decoding and actual
authentication. A readable file proves only retrieval. An explicit adapter can
perform a stronger operation and report its result; the earlier suggestion that
verification can only check presence was unnecessarily restrictive.

Rotation needs an explicit lifetime/refresh policy: systemd supplies an
invocation-scoped copy, and short-lived provider credentials may require a
refreshing provider rather than a one-time static JSON load.

### Kubernetes and other resolvers

Preserve room for a future `kube-secret:` scheme. Its design must identify
cluster context, namespace, Secret name, key and optionally an observed version.
Decide separately whether retrieval uses the Kubernetes API or a projected
volume. Those have different authorization and refresh behavior. Do not freeze
a speculative URI spelling that hides cluster identity or treats mounted paths
as globally meaningful.

Credential resolution can return bytes, a usable provider, or an explicit
unsupported-profile result. Start with bounded bytes for systemd; keep refresh
and provider support in the roadmap rather than forcing every future identity
mechanism into permanent static secret bytes.

## Kinds and MIME types

The inspected IANA application registry has no entry named for SlateDB or
Iceberg. It does include `application/slate`; that is not evidence of a SlateDB
registration and must not be borrowed for this purpose.

Verified representation types:

- Arrow IPC file: `application/vnd.apache.arrow.file`.
- Arrow IPC stream: `application/vnd.apache.arrow.stream`.
- Parquet file: `application/vnd.apache.parquet`.

Use `kind: "slatedb"` for a database and leave its media type absent unless
describing a specific encoded artifact. Local kind names need a documented
namespace/profile for interchange; they do not imply upstream endorsement.

Likewise, an Iceberg table is more than a Parquet file. A useful future record
may identify a catalog, namespace/table, and snapshot, with separate storage
bindings. Arrow describes representations and computation interfaces; it does
not by itself identify a lakehouse checkout. “Lakehouse” remains a broad label
until a concrete consumer and its access model are selected. The user's “yada”
driver reference remains unidentified; do not silently substitute a project.

## Filesystem organization

Two distinct layouts need deciding: persisted user records and source modules.

### Persisted records

Recommended initial layout is a caller-selected catalog directory:

```text
catalog/
  resources/
    catalog.json
    analytics.json
  credentials/
    catalog-creds
  connections/
    local-rustfs.json
  realizations/
    analytics-local.json
```

`catalog-creds` is UTF-8 URI text with an optional terminal newline. A rich
credential document instead uses `.json`; consumers use the explicit `ref`, not
basename guessing. The `-creds` suffix is readable convention, not required
magic. Credentials here are references; actual secret payloads remain with the
selected credential provider.

A connection profile is optional: small records embed it, while several
resources can share one by reference. No global catalog daemon is necessary.
The same resource description can be passed as bytes or held in an existing
sealed memfd. fdstore would retain the description carrier, not the S3 database.

An alternative is resource-local grouping, such as `catalog/db/resource.json`
and `catalog/db/creds`. It is convenient for self-contained exports but makes
sharing less direct. Prefer the domain-grouped layout initially; allow portable
bundles to contain their explicit reference closure.

Persistence work must define record versioning, stable IDs versus filenames,
relative-reference bases, missing/duplicate IDs, bounded cycle detection, and
atomic single-document replacement. Cross-file consistency requires a selected
catalog revision or a combined export when it matters; directory enumeration
alone does not create a transaction. Preserve unknown optional data on editing.
Cleanup remains deliberate.

### Possible source homes

```text
resources/core/          record vocabulary and validation
resources/catalog/       filesystem reading/writing and reference resolution
credentials/core/        reference vocabulary and resolution outcomes
credentials/systemd/     credential-directory adapter
credentials/kubernetes/  future resolver
storage/object-store/    provider/connection configuration adapter
storage/slatedb/         database-specific opening and lifecycle
datasets/relations/      local realization/revision vocabulary
examples/remote-slate/   first composed demonstration
fixtures/rustfs/         opt-in real S3 service fixture
```

These are proposed responsibility boundaries, not a request to scaffold empty
crates. Another plausible grouping puts provider adapters under
`resources/adapters/` and keeps only the reusable credential domain separate.
Prefer the domain split above if independent consumers justify the packages.
Heavy providers, SlateDB, runtimes and network clients remain opt-in; the current
descriptor-only `HandleTable<H: AsFd>` need not become a universal resource enum.

## Local mirrors and checkouts

A realization record relates a local locator to an upstream resource:

```json
{
  "version": 1,
  "id": "analytics-local",
  "kind": "realization",
  "uri": "file:///srv/data/analytics",
  "relation": "checkout-of",
  "source": { "ref": "../resources/analytics.json" },
  "source_revision": { "scheme": "adapter-defined", "value": "revision-42" },
  "coverage": "unknown",
  "labels": { "owner": "analysis" }
}
```

Allow `mirror-of`, `checkout-of`, `cache-of` and `derived-from`, with distinct
semantics. Copying a few Parquet files is not necessarily a complete table
checkout. Record revision, coverage, local modifications and observation time
where known. “Unknown” remains distinct from complete/current. A checkout's
source identity and its local location must both survive serialization.

First implement declarations and inspection. Later adapters can establish
consistent snapshots, download closure, freshness or synchronization. Automatic
sync is a separate capability, not an implied effect of declaring a relation.

## SlateDB and fixture evidence

The local SlateDB `Db::resolve_object_store` requires a bucket-level URL and
accepts environment-derived provider options. Database builders receive the
path separately. An adapter can instead construct an explicit object store
from connection settings and resolved credentials, without mutating process
environment. Audit separate WAL-store configuration before claiming a complete
reopen recipe for deployments using it.

Reopening durable data simplifies custody, but does not eliminate lifecycle.
`test_empty_wal_should_fence_old_writer` demonstrates that opening another writer
fences the original. Thus discovery and descriptive validation must not call
writer-open as a probe. Determine reader versus writer intent; open a writer
only at the selected takeover step. Check write durability/close behavior and
pending operations before promising continuity. A URI preserves none of the
old process's transactions, pending requests or uncommitted memory by itself.

For fixtures, the direct fit is SlateDB's existing `object_store` abstraction:

1. In-memory store for adapter behavior inside one process.
2. Local filesystem store for persisted reopen across processes.
3. RustFS as an opt-in S3-compatible service, reached through the existing S3
   client/provider with explicit endpoint and test credentials.

RustFS advertises standalone and container deployment. A targeted local source
search found internal mocks, but did not establish a public embeddable dummy
driver implementing SlateDB's `object_store` trait. Do not depend on that
unverified facility. A pinned RustFS service fixture can test real HTTP/auth
behavior; its compatibility with the exact conditional-write/list/range operations
used by the pinned SlateDB version still needs execution evidence.

Fixture tooling should create an isolated bucket/prefix, wait for readiness,
provision disposable test credentials, report versions, and expose deliberate
teardown. Unit tests need no running daemon. A live systemd fixture must be
reported separately from a test directory standing in for credentials.

## Proposed epics and tickets

These IDs are planning candidates. Search the native tracker before creating
them; link the design and add explicit parent-child edges at materialization.

| Epic | Outcome | Proposed children and acceptance |
| --- | --- | --- |
| `handoff-resources` | Descriptions that can be passed and interpreted without FDs | `-schema`: define kinds, URI context, optional fields, extensions and example records; `-catalog`: round-trip files and tiny references, reject ambiguity/cycles, preserve optional data; `-delivery`: pass one description via existing bytes/memfd mechanisms and resolve it in a second process |
| `handoff-credentials` | Reusable credential references independent of SlateDB | `-uri`: define compact/rich/ref forms and role bindings; `-systemd`: read bounded credential bytes with name validation, distinguish missing context/name/invalid data; `-profiles`: define static AWS test profile, decode without environment mutation, specify refresh limits; `-kubernetes-design`: settle cluster/namespace/key identity and API versus volume paths |
| `handoff-object-store` | Explicit provider configuration and reproducible test services | `-adapter`: build memory/local/S3 stores from scoped connection data and credential inputs; `-rustfs-fixture`: pinned isolated service, readiness/provisioning/teardown; `-conformance`: exercise required conditional writes, listing, range reads and authentication through the actual client |
| `handoff-slatedb` | A resource record can reopen the intended durable database | `-profile`: define prefix/escaping, options, reader/writer intent and WAL-store completeness; `-reopen`: cross-process local persistence with known durable values; `-s3-systemd`: compose RustFS and real LoadCredential, proving declared binding use; `-takeover`: test old-writer fencing and selected release ordering, with explicit durability boundaries |
| `handoff-realizations` | Express local copies in relation to remote origins | `-model`: source/local locators, revision, coverage and dirty-state semantics; `-catalog`: persist and query relationships without fetching; `-snapshot-design`: select one backend and specify a verifiable consistent-copy operation |
| `handoff-datasets` | Extend resource descriptions to concrete table/representation consumers | `-kinds`: document Arrow/Parquet media types versus dataset kinds; `-consumer-discovery`: identify the intended driver and choose the first table/catalog system; `-iceberg-profile`: after selection, model catalog/table/snapshot and storage bindings with interoperability examples |

Dependencies are capability-sized: schema and credential URI vocabulary first;
catalog and systemd resolver can then advance independently. Object-store
adapter plus SlateDB profile enables local reopen. RustFS conformance and
credential profiles enable the S3/systemd composition. Takeover follows a
working reopen path. Realization declarations need schema/catalog but not S3;
dataset consumer discovery can proceed independently.

**Recommended first coherent slice:** resource schema, credential URI forms,
filesystem examples, and the systemd byte resolver. Then a local SlateDB reopen
example; then RustFS plus real systemd credentials. This establishes the tiny
reference first while keeping the larger S3 and dataset ambitions visible.

## Connections and verification

- [Names-first adoption](../fd-handoff/systemd-native-adoption0.gpt6a.md) supplies
  the precedent for small application-interpreted records in independent
  carriers. Remote descriptions extend that approach beyond live descriptors.
- [Original architecture](../fd-handoff/architecture0.gpt6a.md) supplies the
  opt-in composition rule. New resource/credential domains should preserve it.
- [Current workspace](../../README.md) distinguishes implemented mechanics
  from the proposed adapters here; [the work map](../fd-handoff/work.md) retains
  the existing E1–E11 undertakings.

Evidence here is source/document inspection, not a SlateDB/RustFS/systemd
integration run. Local source citations identify inspected checkout files,
not immutable revisions. URI schemes, JSON fields, filesystem layout, crate
boundaries and ticket scope are proposals for the next decision.

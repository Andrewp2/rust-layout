# Glassworks Collaboration Model

Glassworks collaboration is moving toward a local-first CRDT design. The current implementation uses [Loro](https://loro.dev/) as the replicated update log while keeping Glassworks's own semantic CAD commands as the editor-facing operation language.

## Current Slice

The editor still represents changes as semantic `Operation` commands: add shape, move shape, replace shape, rename cell, move instance, and so on. Those commands are wrapped in `CrdtOperation` envelopes:

```text
CrdtOperation
  id: actor UUID + per-actor counter
  deps: observed actor clocks when the command was created
  operation: semantic layout command
```

The document tracks compatibility metadata:

- `crdt_seen`: operation IDs already applied, so duplicate delivery is idempotent.
- `crdt_actor_clocks`: highest observed counter per actor.
- `crdt_operation_log`: accepted CRDT operations for replay, snapshots, and future persistence.

Loro owns the replicated transport and object-store state:

- Native clients append each semantic operation to a Loro list as JSON.
- The same update writes object state into Loro maps:
  - `glassworks_shapes`: one child map per `ShapeId`.
  - `glassworks_cells`: one child map per `CellId`.
  - `glassworks_instances`: one child map per `(parent CellId, InstanceId)`.
- Object maps use LWW register fields for IDs, names, layers, net IDs, shape-kind JSON, and transforms.
- Object deletion is represented as a `deleted` tombstone register instead of removing the map entry.
- Clients export incremental Loro update bytes and send those over WebSocket.
- The sync server imports Loro updates, extracts newly observed semantic operations, applies them to the authoritative `Document`, and broadcasts the same Loro update.
- Snapshots include both the current `Document` and the Loro snapshot bytes, so reconnecting clients can avoid re-emitting old operations.
- The existing `Document` is now a materialized render/edit view. The model layer can rebuild shape/cell/instance objects from the Loro maps, but the app still applies semantic operations incrementally for the hot editing path.

## Undo Semantics

Undo is not a raw history rewrite. It is a new semantic command that compensates for a previous local command.

Examples:

```text
MoveShape(+100, 0) undo -> MoveShape(-100, 0)
AddShape(shape 42) undo -> DeleteShape(42)
ReplaceShape(old -> new) undo -> ReplaceShape(new -> old)
```

This matches the direction we want for collaborative CAD: users undo their own semantic actions, and those undo actions are themselves replicated operations.

## What This Does Not Solve Yet

This is not yet a full CAD object CRDT. Loro handles replication, update encoding, duplicate update import, snapshots, object-map registers, and tombstone storage. Glassworks still needs more domain-specific merge semantics for layout objects. The current slice does not provide:

- Geometry-level CRDT lists for polygon/path vertices.
- Multi-value conflict display for fields such as names, layers, and transforms.
- Selective undo of arbitrary older remote operations.
- Offline queues and reconnect reconciliation.

Those pieces should be added at the layout-model boundary before leaking collaboration semantics into renderer or UI code.

## Next CRDT Steps

1. Persist the server Loro snapshot/update log and load it on startup.
2. Use the Loro materializer for reconnect recovery and persisted server startup.
3. Split polygon/path geometry into Loro-friendly vertex lists with explicit merge policies.
4. Add UI affordances for field conflicts and delete-vs-edit cases.
5. Add native and WASM reconnect tests that replay duplicate and reordered Loro updates.

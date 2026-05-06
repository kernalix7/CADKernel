# Recompute Scheduler

**Status:** spec v1, last updated 2026-05-06
**Owner:** kernel-engineer (`crates/modeling`)
**Implements:** roadmap §8 (history & recompute)
**Crate:** `cadkernel-modeling::history`

---

## 1. Purpose

When a parameter or upstream feature is edited, downstream features must be recomputed. The scheduler:

1. Identifies the **dirty subgraph** — operations whose inputs changed.
2. Topologically orders the dirty operations.
3. Re-evaluates each in dependency order, using cached outputs where possible.
4. Validates the resulting B-Rep.
5. Reports per-operation success/failure.

---

## 2. Recompute graph

```
Operation = { id, kind, inputs: [op_id], outputs: [entity_id], parameters, cache_key }

History = DAG of Operations, edges = dependency.
```

Cache key is BLAKE3 hash of:
- Operation kind.
- Sorted input op_ids' cache_keys.
- Canonicalised parameter map (CBOR-encoded, deterministic key order).

If `current_cache_key == stored_cache_key`, output is reused (no re-evaluation).

---

## 3. Dirty propagation

```text
fn mark_dirty(history, edited_param):
    affected_ops ← ops_referencing(edited_param)
    queue ← affected_ops
    dirty ← {}
    while queue nonempty:
        op ← queue.pop()
        if op in dirty: continue
        dirty.insert(op)
        for downstream in history.children(op):
            queue.push(downstream)
    return dirty
```

Complexity: $O(|V| + |E|)$ in graph size.

---

## 4. Recompute pass

```text
fn recompute(history, dirty):
    order ← topo_sort(dirty)
    for op in order:
        new_key ← compute_cache_key(op)
        if new_key == op.cache_key and op.outputs are valid:
            continue                          # cache hit
        try:
            outputs ← execute(op)
            op.cache_key ← new_key
            op.outputs ← outputs
            op.timestamp ← now()
        except err:
            op.state ← Failed(err)
            mark_downstream_failed(op)
            return Partial { failed_at: op, succeeded: ... }
    validate_brep(history.final_state)
    return Success
```

---

## 5. Failure handling

| Failure | Behaviour |
|---|---|
| Operation throws | Mark op as `Failed`; downstream ops marked `Suppressed`; UI shows red icon |
| Validation fails after success | Last-good snapshot restored; user offered "rollback" or "force keep" |
| Cycle detected in dependency | Reject edit; report cycle path to user |
| Cache key collision | Use full BLAKE3 (not truncated) — collision astronomically unlikely |
| Recompute timeout | Abort current op; restore prior state |

---

## 6. Performance budgets

| Scenario | Target |
|---|---|
| Single param edit, dirty count = 1 | $< 50$ ms |
| Sketch dimension drag (continuous) | $< 16$ ms per frame |
| Full rebuild from scratch (50 features) | $< 2$ s |
| Full rebuild (500 features, industrial) | $< 30$ s |

---

## 7. Concurrency

- Independent dirty sub-DAGs recomputed in parallel via rayon.
- Each operation runs on a single thread (no intra-op parallelism unless op declares `parallel: true`).
- Cache reads are lock-free (read-only after build); writes use per-op `RwLock`.

---

## 8. Persistent cache

Cache survives across sessions: stored in `.cadkernel-cache/` next to the `.cadk` file. Keyed by:
- Cache key.
- File modification time of `.cadk`.

On file open, cache pre-loaded; outputs not re-executed unless cache miss.

---

## 9. Acceptance gates

- v1.0: edit param, downstream rebuilds correctly for 100 hand-crafted models.
- v2.0: 500-feature industrial part fully rebuilds in $< 30$ s.
- v3.0: incremental edit averages $< 50$ ms across 1000 trace samples.

---

## 10. References

1. Reps, Teitelbaum. "The synthesizer generator." *ACM SIGPLAN Notices* 1984. (Incremental computation foundations.)
2. Bezem, Klop. "Term Rewriting Systems." Cambridge, 2003.
3. SolidWorks Knowledge Base: "Understanding the rebuild process."
4. OCCT XCAF documentation.

# loken-cudarc — patch inventory & road to zero-fork

LOKEN uses [cudarc](https://github.com/chelsea0x3b/cudarc) (the official crate; the repo /
crates.io owner renamed `coreylowman` → `chelsea0x3b`, same project). It is **vendored**
in the main repo under `libs/cudarc` as **cudarc 0.19.9 + a small local patch set**.

**Goal: eliminate the fork** — get to a plain `cudarc = "…"` crates.io dependency and delete the
vendored copy. This file tracks every local modification, why it exists, its upstream status, and
how to remove it.

## Base version

- Vendored base: **0.19.9** (bumped from 0.19.4 on 2026-08-12).
- The three patches are regenerated against pristine 0.19.9 and verified to reapply cleanly to
  it, so the vendored tree is exactly `pristine + patches/` — check that invariant after every
  bump, it is what makes the fork auditable.
- What the bump cost: `0002` applied untouched, `0003` needed one hunk of its eleven rebased
  (upstream wrapped the stream waits in `Drop for CudaSlice` behind
  `is_managing_stream_synchronization()`), and `0001` LOST HALF ITS REASON TO EXIST — see below.

## The patch set

Only **5 files** differ from pristine cudarc; one is noise. Real changes = **3 patches** (see
`patches/`), all generated vs pristine 0.19.9.

| # | File(s) | What | Why | Upstreamable? | Eliminable in LOKEN? |
|---|---------|------|-----|---------------|----------------------|
| — | `src/cudnn/mod.rs` | none — CRLF→LF only | vendoring artifact | n/a | **gone** (not reintroduced at 0.19.9) |
| — | `build.rs` cuDNN path | a hardcoded `C:/…/CUDNN/v9.20` | Windows cuDNN discovery | **absorbed** | **gone**: 0.19.9 derives `v{major}.{minor}`, strictly more general than ours. Applying the old patch with fuzz inserted that literal INSIDE a `#[cfg(any(…))]` list and broke the build — a patch that survives a bump by fuzzing is not the same patch. |
| 0001 | `build.rs` (+20/-1) | (a) forward-compat CUDA fallback: a toolkit newer than the newest supported bindings falls back to the newest ≤ it instead of `panic!` (needed for CUDA 13.3 → 13.2 bindings to enable NCCL/tensor-parallel); (b) adds CUDNN Windows path `v9.20` | build against newer CUDA/CUDNN than cudarc pins | **yes — clean generic PR** | via upstream |
| 0002 | `src/driver/safe/graph.rs` (+227), `src/driver/safe/mod.rs` | CUDA-graph node introspection (`num_nodes`, `nodes`, `node_types`) + `UpdateOutcome` graph-node patching | inspect/patch captured graphs (fused-kernel / graph work) | **yes — useful introspection API PR** | via upstream |
| 0003 | `src/driver/safe/core.rs` (+294, 11 hunks) | `force_sync_alloc` (force `cuMemAlloc` over the async pool), a **capture arena** (bump-allocate from a pre-reserved region during graph capture), and **deferred-free** fences | the stream-ordered async allocator (`cuMemAllocAsync`) causes a use-after-free on long decode, and allocations *during* graph capture bake MEM_ALLOC/FREE nodes into the graph → `CUDA_ERROR_ILLEGAL_ADDRESS` on replay | hard (invasive/opinionated) | **YES — move to a LOKEN-side allocator on stock cudarc** (see below) |

## Key finding — core.rs does NOT require a cudarc patch

cudarc exposes the raw **synchronous** allocation primitives publicly:
`cudarc::driver::result::malloc_sync` (`cuMemAlloc`) and `result::free_sync` (`cuMemFree`).
The only reason we patched cudarc is that its *safe* path (`CudaStream::alloc` → `CudaSlice`) forces
`cuMemAllocAsync` on async-capable devices, with no override (`has_async_alloc` is an immutable
detected bool as of 0.19.8; upstream PR #486 only touches the sync-*free* fallback).

Therefore the entire 0003 patch can be replaced **inside LOKEN** by owning device allocation on
top of stock cudarc:
- **sync alloc** → allocate via `result::malloc_sync` / free via `result::free_sync` (bypasses the
  async pool → removes the use-after-free without patching cudarc);
- **arena** → one big `malloc_sync` region, bump sub-allocation in LOKEN code (also makes graph
  capture replay-safe: no alloc/free during capture);
- **deferred-free** → defer the `free_sync` calls in LOKEN.

Cost: a refactor of LOKEN's CUDA tensor backend allocation layer (a LOKEN-owned device-buffer
type instead of `CudaSlice`), **not** a cudarc patch.

## Road to zero-fork

1. ~~Drop the cudnn line-ending diff (no-op).~~ **Done** — not reintroduced at 0.19.9.
2. ~~**Bump** the vendored base and rebase 0001–0003.~~ **Done** — 0.19.4 → 0.19.9 on
   2026-08-12, and the bump absorbed the cuDNN path patch upstream.
3. **Upstream** 0001 (build.rs forward-compat) and 0002 (graph introspection) to
   `chelsea0x3b/cudarc`. When merged → delete those patches.
4. **Refactor** LOKEN's CUDA allocation onto stock cudarc's `result::malloc_sync`/`free_sync`
   (+ LOKEN-side arena) → delete 0003.
5. **End state:** no patches left → replace `libs/cudarc` with a plain crates.io
   `cudarc = "0.19.x"` dependency and **delete this repo** (or keep it as history).

## Applying the patches (until eliminated)

From a pristine cudarc 0.19.9 source tree:

```bash
for p in patches/*.patch; do patch -p1 < "$p"; done   # or: git apply patches/*.patch
```

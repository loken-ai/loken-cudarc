# Notice

**This repository is not original work.** It redistributes
[**cudarc**](https://github.com/chelsea0x3b/cudarc) by Chelsea Lowman, under cudarc's own
terms — **MIT OR Apache-2.0** — with a small local patch set on top.

| Path | What it is | Whose it is |
|------|------------|-------------|
| `cudarc/` | The vendored upstream sources, at **0.19.9** | cudarc's authors. Upstream's own licence files travel with them, unmodified, at `cudarc/LICENSE-MIT` and `cudarc/LICENSE-APACHE` |
| `patches/` | The local delta — three patches | The contributors to this repository |
| `PATCHES.md`, `README.md`, `CHANGELOG.md` | Documentation of that delta | The contributors to this repository |

**No copyright is asserted over the vendored sources.** The patches and the documentation are
the only original content here, and they are offered under the **same MIT OR Apache-2.0 terms
as upstream**, so that any of them can be sent upstream without raising a licence question.

**Which files are modified, and why**, is enumerated in [`PATCHES.md`](./PATCHES.md): five files
of the vendored tree differ from pristine cudarc, of which three carry real changes. The
vendored tree is exactly `pristine + patches/`, and that invariant is re-checked after every
version bump — it is what makes the delta auditable rather than merely claimed.

The purpose of this repository is to **stop existing**. Every patch is tracked with its upstream
status and its route to removal; the end state is a plain `cudarc` dependency from crates.io and
no fork at all.

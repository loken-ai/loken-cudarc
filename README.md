# loken-cudarc

Transitional home for LOKEN's local delta over [**cudarc**](https://github.com/chelsea0x3b/cudarc)
(the official crate). LOKEN vendors cudarc **0.19.9 + a small patch set**; this repo tracks those
patches and the plan to remove them.

This repository carries the patched sources under `cudarc/`, so LOKEN can depend on it directly.

**The goal is to make it unnecessary** - reach zero local patches, depend on the published
`cudarc`, and delete the copy. The compiler already states the current gap: LOKEN does not
build against a stock `cudarc`, because graph-node introspection (patch 0002) is not upstream.

See **[`PATCHES.md`](./PATCHES.md)** for the full inventory (3 real patches, all eliminable) and the
road to zero-fork. Patch files live in [`patches/`](./patches/).

## Licensing

cudarc is not ours. This repository redistributes it under its own **MIT OR Apache-2.0** terms,
asserts no copyright over those sources, and offers the local patches under the same terms so
they can go upstream freely. See **[`NOTICE.md`](./NOTICE.md)**.

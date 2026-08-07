# Changelog

All notable changes to this repository. The format follows [Keep a Changelog](https://keepachangelog.com/1.1.0/).

## [0.1.0]

### Added

- An inventory of every local modification LOKEN carries over `cudarc` 0.19.4, with the
  reason each exists, its upstream status, and what has to happen for it to go away
  (`PATCHES.md`).
- The three patches themselves, as applicable files under `patches/`.

Only five files differ from a pristine `cudarc`, and one of those is a line-ending change with
no effect. Two of the three real patches are upstreamable as they stand. The third works
around a use-after-free in the asynchronous memory pool and makes graph capture safe to
replay - and it needs no patch at all, because `cudarc` already exposes the synchronous
allocation entry points it relies on: the whole arena and deferred-free scheme can live in
the consumer's allocator instead.

This repository exists to be deleted. Success is `cudarc = "0.19.x"` from crates.io, no
vendored copy, and nothing here.

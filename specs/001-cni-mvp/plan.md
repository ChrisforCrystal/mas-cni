# Implementation Plan: Initial CNI MVP in Go

**Branch**: `001-cni-mvp` | **Date**: 2026-01-10 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-cni-mvp/spec.md`

## Summary

Implement a minimal CNI plugin (`mascni`) in Go that supports the `ADD`, `DEL`, `CHECK`, and `VERSION` commands. The plugin will create a bridge on the host, connect containers via veth pairs, and delegate IPAM to `host-local`.

## Technical Context

**Language/Version**: Go 1.23+
**Primary Dependencies**:

- `github.com/containernetworking/cni`
- `github.com/containernetworking/plugins`
  **Target Platform**: Linux (Kernel 5.4+ recommended for Phase 2, but minimal for Phase 1)
  **Project Type**: Single CLI Binary
  **Performance Goals**: < 500ms startup
  **Constraints**: Must run as root

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

- [x] **Iterative Implementation (MVP First)**: Yes, pure Go MVP.
- [x] **CNI Specification Compliance**: Yes, standard library usage ensures this.
- [x] **Test-Driven Verification**: `cnitool` integration planned.
- [x] **Safety & Reliability**: Standard error handling.
- [x] **Simplicity**: Delegating IPAM, using standard bridge.

## Project Structure

### Documentation (this feature)

```text
specs/001-cni-mvp/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── cni-config.json  # Example config serving as contract
└── tasks.md
```

### Source Code

```text
.
├── cmd/
│   └── mascni/
│       └── main.go       # Entry point (skel.PluginMain)
├── pkg/
│   ├── config/           # Configuration parsing
│   ├── masipam/          # IPAM delegation wrapper
│   └── masnet/           # Network logic (Bridge, Veth)
├── go.mod
└── go.sum
```

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
| --------- | ---------- | ------------------------------------ |
| None      | N/A        | N/A                                  |

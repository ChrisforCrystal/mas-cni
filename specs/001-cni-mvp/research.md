# Research: Initial CNI MVP

## Decisions & Rationale

### 1. Project Structure

- **Decision**: Use standard Go CLI structure with `cmd/mascni/main.go` and `pkg/` for logic.
- **Rationale**: Keeps the entry point minimal and logic testable. `pkg/mascni` will contain the core `cmdAdd`/`cmdDel` implementations.

### 2. IPAM Integration

- **Decision**: Use `host-local` via delegation (exec).
- **Rationale**: Follows standard CNI composability patterns. Using the `host-local` binary via `github.com/containernetworking/cni/pkg/invoke` is simpler and more robust than embedding the library codebase, ensuring consistent behavior with standard K8s setups.

### 3. Namespace & Networking Library

- **Decision**: Use `github.com/containernetworking/plugins/pkg/ns` and `github.com/containernetworking/plugins/pkg/ip`.
- **Rationale**: These are the battle-tested libraries used by the reference `bridge` plugin. They handle the complexities of entering namespaces (`ns.WithNetNSPath`) and setting up veth pairs (`ip.SetupVeth`).

### 4. Code Structure for Optimizations (Phase 2 Prep)

- **Decision**: Define a `Datapath` interface.
- **Rationale**: Phase 1 will implement `Datapath` using Linux Bridge (`netlink`). Phase 2 will create a new implementation using eBPF, making the transition smoother.

## Outstanding Questions (Resolved)

- **Q: How to invoke IPAM?**
  - **A**: Use `ipam.ExecAdd` from `github.com/containernetworking/cni/pkg/invoke`.

## dependencies

- `github.com/containernetworking/cni` (v1.1.2+)
- `github.com/containernetworking/plugins` (v1.3.0+)
- `github.com/vishvananda/netlink` (transitive, but verifying version is good)

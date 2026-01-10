# Feature Specification: Initial CNI MVP in Go

**Feature Branch**: `001-cni-mvp`
**Created**: 2026-01-10
**Status**: Draft
**Input**: User description: "开始设计 MVP 具体功能吧" (Design MVP specific features)

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Pod Connectivity (ADD) (Priority: P1)

As a Kubernetes node (Kubelet) or Container Runtime, I want to invoke the CNI plugin to add a network interface to a container so that the container can communicate with the host and other containers.

**Why this priority**: Fundamental requirement. Without this, the plugin does nothing.

**Independent Test**: Can be tested using `cnitool` or manual `ip netns` commands without a full K8s cluster.

**Acceptance Scenarios**:

1. **Given** a valid CNI configuration referencing the plugin and `host-local` IPAM, **When** `CNI_COMMAND=ADD` is executed with a target network namespace, **Then** an interface (e.g., `eth0`) exists in the target namespace with a valid IP address and default route, and the host side veth is attached to the specified bridge.

---

### User Story 2 - Pod Cleanup (DEL) (Priority: P1)

As a Container Runtime, I want to invoke the CNI plugin to remove a network interface when a container is stopped so that resources (IPs, interfaces) are released.

**Why this priority**: Essential for preventing resource leaks and ensuring long-running node stability.

**Independent Test**: Create a network namespace and interface/IP, then run DEL.

**Acceptance Scenarios**:

1. **Given** a container namespace previously set up by the plugin, **When** `CNI_COMMAND=DEL` is executed, **Then** the container interface is removed, the host side veth is removed, and the allocated IP is released (verified via IPAM).

---

### User Story 3 - Configuration Verification (CHECK/VERSION) (Priority: P2)

As an Operator or Deployment Tool, I want to verify the plugin capabilities and health so that I can ensure the node is ready for workloads.

**Why this priority**: Required by CNI specification v0.4.0+ and K8s readiness checks.

**Independent Test**: Run `CNI_COMMAND=VERSION` and `CNI_COMMAND=CHECK`.

**Acceptance Scenarios**:

1. **Given** the plugin binary, **When** `CNI_COMMAND=VERSION` is executed, **Then** it outputs supported CNI versions JSON.
2. **Given** a healthy node state, **When** `CNI_COMMAND=CHECK` is executed, **Then** it returns valid success code (0).

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: System MUST implement the CNI Specification v1.0.0 (or compatible latest) interface (stdin/stdout/env vars).
- **FR-002**: System MUST parse CNI configuration from stdin, including `name`, `cniVersion`, and IPAM delegation fields.
- **FR-003**: System MUST support the `bridge` network model: creating a Linux Bridge on the host if it doesn't exist.
- **FR-004**: System MUST create veth pairs connecting the container namespace to the host bridge.
- **FR-005**: System MUST delegate IP Address Management (IPAM) to the standard `host-local` plugin.
- **FR-006**: System MUST configure the container's default route to point to the bridge gateway.
- **FR-007**: System MUST handle `cmdDel` idempotently (succeed even if interface is already gone).

### Key Entities _(include if feature involves data)_

- **NetworkConfig**: The JSON configuration provided by kubelet, containing bridge name, IPAM config, etc.
- **RuntimeArgs**: The arguments passed via environment variables (CONTAINER_ID, NETNS, IFNAME, etc.).

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: 100% pass rate on standard `cnitool` verification suite (ADD, CHECK, DEL).
- **SC-002**: Ping latency between two containers on the same host is < 1ms (standard veth performance).
- **SC-003**: Plugin binary size is < 20MB (Go optimization target).
- **SC-004**: Successful execution of `cmdAdd` completes in < 500ms under normal load.

## Assumptions

- Target environment is Linux (namespace support required).
- `host-local` binary is available in CNI path for IPAM.
- Root privileges are available (required for network manipulation).

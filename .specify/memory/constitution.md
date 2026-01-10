<!--
Sync Impact Report:
- Version Change: None -> 1.0.0 (Initial Creation)
- Added Principles:
  - Iterative Implementation (MVP First)
  - CNI Specification Compliance
  - Test-Driven Verification
  - Safety & Reliability
  - Simplicity & Maintainability
- Governance: Initial ratification.
- Templates: Checked (plan-template, spec-template, tasks-template). No updates needed.
-->

# mascni Constitution

## Core Principles

### I. Iterative Implementation (MVP First)

The project follows a strict two-phase evolution strategy.
Phase 1: Build a fully functional Minimum Viable Product (MVP) using pure Go. Focus on correctness, CNI compliance, and basic connectivity.
Phase 2: Optimize the data path using Rust and eBPF.
Rationale: Go libraries for CNI are mature and allow rapid prototyping. Optimization should only occur after functional correctness is established.

### II. CNI Specification Compliance

All components must strictly adhere to the Container Network Interface (CNI) specification.
The plugin must support standard CNI verbs (ADD, DEL, CHECK, VERSION) and standard configuration structures at all stages of development.
Tests must verify spec compliance (e.g., using `cnitool`).

### III. Test-Driven Verification

Verification is primary.

- Unit tests for Go logic are required.
- Integration tests (using Kind or local namespaces) must run on every significant change.
- Performance benchmarks must be established before the Rust/eBPF transition to quantify gains.

### IV. Safety & Reliability

Network plugins are critical infrastructure.
Failure modes must be handled gracefully (clean up on failure, no hanging implementations).
Rust components must leverage safety guarantees; Go components must handle errors robustly.

### V. Simplicity & Maintainability

Code should be as simple as possible.
Complex eBPF optimizations must be well-documented and isolated from the control plane logic where possible.

## Technology Stack & Constraints

**Phase 1 (MVP)**

- Language: Go (Latest Stable)
- Libraries: `containernetworking/cni`, `containernetworking/plugins`

**Phase 2 (Optimization)**

- Language: Rust (Latest Stable)
- Technology: eBPF (Extended Berkeley Packet Filter)
- Libraries: `aya-rs` or `cilium/ebpf` (TBD based on evaluation)

**General**

- Target OS: Linux
- Container Runtime: Containerd / Docker

## Development Workflow

1.  **Design**: Update spec/plan before coding.
2.  **Test**: Write verification steps (automated or manual) in `task.md` or `plan.md`.
3.  **Implement**: Code the solution.
4.  **Verify**: Run tests and benchmarks.
5.  **Review**: Self-review or user-review before merging/finalizing.

## Governance

This Constitution defines the non-negotiable rules for the `mascni` project.

- **Supremacy**: These principles supersede other guidelines or preferences unless explicitly amended.
- **Amendments**: Changes to this document require a formal pull request/update with justification (e.g., changing the technology stack or dropping a principle).
- **Compliance**: All artifacts (plans, specs, tasks) must align with these principles.

**Version**: 1.0.0 | **Ratified**: 2026-01-10 | **Last Amended**: 2026-01-10

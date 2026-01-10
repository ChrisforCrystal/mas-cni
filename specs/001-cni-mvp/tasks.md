---
description: "Task list for CNI MVP implementation"
---

# Tasks: Initial CNI MVP in Go

**Input**: Design documents from `/specs/001-cni-mvp/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md

**Organization**: Tasks are grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel
- **[Story]**: [US1] Pod Connectivity, [US2] Pod Cleanup, [US3] Verification
- File paths are relative to repository root

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [x] T001 Initialize Go module `github.com/masallsome/mascni`
- [x] T002 [P] Create directory structure (`cmd/mascni`, `pkg/config`, `pkg/masnet`, `pkg/masipam`)
- [x] T003 Install CNI dependencies (`containernetworking/cni`, `plugins`, `vishvananda/netlink`)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T004 Implement configuration parsing in `pkg/config/types.go` and `pkg/config/conf.go` (maps to `data-model.md`)
- [x] T005 Implement `skey.PluginMain` harness in `cmd/mascni/main.go`
- [x] T006 Define `Datapath` interface for future extensibility in `pkg/masnet/datapath.go`

**Checkpoint**: Binary builds and accepts CNI config JSON (even if it does nothing).

---

## Phase 3: User Story 1 - Pod Connectivity (ADD) (Priority: P1) 🎯 MVP

**Goal**: Invoke plugin to add a network interface to a container.

**Independent Test**: Use `cnitool add ...` and verify interface creation.

### Implementation for User Story 1

- [x] T007 [P] [US1] Implement Linux Bridge creation logic in `pkg/masnet/bridge.go`
- [x] T008 [P] [US1] Implement Veth pair creation logic in `pkg/masnet/veth.go`
- [x] T009 [US1] Implement IPAM delegation (Add) in `pkg/masipam/ipam.go`
- [x] T010 [US1] Wire `cmdAdd` logic in `cmd/mascni/main.go` using `masnet` and `masipam`
- [x] T011 [US1] Implement result generation (CNI JSON) in `cmd/mascni/main.go`

**Checkpoint**: `cnitool add` succeeds, interface exists, IP is assigned.

---

## Phase 4: User Story 2 - Pod Cleanup (DEL) (Priority: P1)

**Goal**: Remove network interface and release IP when container stops.

**Independent Test**: Use `cnitool del ...` and verify cleanup.

### Implementation for User Story 2

- [x] T012 [P] [US2] Implement Bridge/Veth cleanup logic (if needed) in `pkg/masnet/cleanup.go`
- [x] T013 [US2] Implement IPAM delegation (Del) in `pkg/masipam/ipam.go`
- [x] T014 [US2] Implement `cmdDel` logic in `cmd/mascni/main.go` (ensure idempotency)

**Checkpoint**: `cnitool del` succeeds, IP released.

---

## Phase 5: User Story 3 - Configuration Verification (CHECK/VERSION) (Priority: P2)

**Goal**: Verify plugin capabilities and health.

**Independent Test**: `cnitool check` and `version` commands.

### Implementation for User Story 3

- [x] T015 [US3] Implement `cmdCheck` logic in `cmd/mascni/main.go`
- [x] T016 [US3] Ensure `VERSION` is handled by `skel.PluginMain`

**Checkpoint**: All CNI verbs supported.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [x] T017 [P] Add README.md for the plugin
- [x] T018 Run `go fmt` and `go vet`
- [x] T019 Verify `quickstart.md` steps manually

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: Start immediately
- **Foundational (Phase 2)**: Depends on Setup
- **User Stories (Phase 3+)**: Depend on Foundational

### User Story Dependencies

- **US1 (ADD)**: Independent
- **US2 (DEL)**: Independent (can be implemented alongside ADD, but testing usually follows ADD)
- **US3 (CHECK)**: Independent

### Parallel Opportunities

- **T002, T007, T008, T012**: Netlink/system wrappers can be built in parallel with config parsing.
- **T009, T013**: IPAM logic can be built parallel to Network logic.

# Tasks

- [x] Update Constitution for `mascni`
- [x] Create Implementation Plan for `001-cni-mvp`
- [x] Implement CNI MVP (Phase 1-6)
- [x] **Verify & Understand MVP**
  - [x] Code Walkthrough (Main, Net, IPAM)
  - [x] Manual Verification (Manual cnitool)
  - [x] Integration Verification (Real K8s Pod)
- [x] **Phase 2: Rust/eBPF Optimization**

  - [x] Create Rust Project Structure (`cargo new`)
  - [x] Implement CNI Skeleton (Rust)
    - [x] Distribute (Build in Docker)
    - [x] Deploy to Kind
    - [x] Verify
  - [x] IPAM Delegation
  - [x] Network Plumbing (Veth/Netns via `ip` command wrapper)
  - [x] **Implement eBPF Datapath**
    - [x] Write eBPF Program (TC Classifier / XDP)
    - [x] Userspace Loader (Shell out to TC/BPFtool)
  - [x] Verify functionality
    - [x] Ping Gateway (Success with 10.88.0.0/16)
    - [x] Verify Host Routes

- [ ] **Phase 3: Advanced Networking (TODO)**
  - [ ] **Code Review & Deep Dive** (User to read `architecture_review.md`)
  - [ ] **Intra-Node Optimization**
    - [ ] Implement `Map-in-Map` for efficient route storage
    - [ ] Implement Direct Pod-to-Pod Redirect (L3 bypass)
  - [ ] **Cross-Node Networking (Big Feature)**
    - [ ] Design Header/Control Plane
    - [ ] Implement Overlay (VXLAN) or Routing
  - [ ] **Refactoring**
    - [ ] Replace `std::process::Command` (ip/tc) with native Rust crates (`rtnetlink`, `aya`/`libbpf-rs`)

# Roadmap

## Phase 1: The Secure Foundation (Architecture & API)

*Establish the technical backbone with security and observability from day one.*

### 1.1 Project Setup & Tooling

- [x] **Init Rust Workspace:** Run `cargo new` with the structure: `core` (Types), `api` (Web Server), `adapters` (OS Interaction).
- [x] **Build System:** Create a `justfile` for commands like `just run` or `just build-ui`.
- [x] **Logging & Tracing:**
  - [x] Configure `tracing` and `tracing-subscriber`.
  - [x] Configure **OpenTelemetry (OTel)** layer.
  - [x] Enable structured JSON logging.
- [x] **Observability Endpoints:** Implement `/healthz` and `/readyz` for uptime/status checks.

### 1.2 Secure Web Server & API Spec

- [x] **TLS Bootstrap:** Integrate `rcgen` crate. On startup, check for `cert.pem`; if missing, generate a self-signed certificate.
- [x] **HTTPS Enforcement:** Configure Axum to listen on 443 (or 8443) with `rustls`. Redirect HTTP to HTTPS.
- [x] **Certificate Storage:** Persist `cert.pem`/`key.pem` at `/etc/opt/storage-os/tls` (mode 600) and allow reload when replaced by user.
- [x] **OpenAPI (Swagger):**
  - [x] Integrate `utoipa` crate.
  - [x] Configure `/swagger-ui` route.
- [x] **Error Handling:** Define a global `AppError` enum mapping to HTTP status codes.

### 1.3 Authentication (PAM & Cookies)

- [x] **PAM Binding:** Integrate `pam` crate for system user authentication.
- [x] **Session Management:**
  - [x] Implement **HttpOnly, Secure, SameSite=Lax** cookies using `tower-sessions` (MemoryStore).
  - [x] Implement CSRF protection middleware.
  - [x] Abstract session storage so MemoryStore can be swapped for file/redis without handler changes.
- [x] **Login Endpoint:** `POST /api/login` (Validates PAM, creates Session, sets Cookie).
- [x] **Frontend Auth:** Svelte 5 store for session state (relying on browser cookie handling).

### 1.4 Service Integration

- [x] **Service User:** Define the user (e.g., `webservd`) the binary will run as.
- [x] **SMF Manifest:** Create the XML manifest to manage the Rust binary as a service (`svccfg import`), set least `privileges`/`limit_privileges`, and ensure TLS key presence as a dependency.
- [x] **Readiness:** `/readyz` must verify TLS material is loaded and session storage is available.

---

## Phase 2: System Management & Reconciliation

*Develop the management tools and ensure UI state matches System state.*

### 2.1 Reconciliation & Actors

- [x] **File Watchers:** Implement `notify` crate to watch `/etc/` for external changes.
- [x] **Global State Sync:** Push "External Change Detected" events to the UI via WebSocket (`/api/v1/events`).
- [x] **PkgActor:** Create an Actor to serialize package operations (prevent parallel updates).
- [x] **NetworkActor:** Actor to serialize `ipadm`/`dladm` calls (read implemented, write stubbed).
- [ ] **SMF/ZFS Polling:** Periodically poll `svcs -H`/`svcs -xv` for managed FMRIs and `zpool status`/`zpool list` to detect external changes; feed results into actor messages.

### 2.2 The Lifesaver: Secure Web Shell

- [x] **Backend:** Spawn `ttyd` as a subprocess.
- [x] **Proxy:** Implement Secure WebSocket (WSS) upgrade in Axum and pipe to `ttyd`.
- [x] **Security:** Validate Session Cookie during WebSocket handshake.
- [x] **Privilege Boundary:** Run `ttyd` under the service user with a constrained env/path; document binary location.
- [x] **UI:** Embed `xterm.js`.

### 2.3 Dashboard & Metrics

- [x] **System Info:** Read Hostname, Kernel, Uptime.
- [x] **Live Metrics:** Push CPU/RAM usage via WSS.
- [x] **UI:** Svelte 5 Runes-based dashboard components.
- [x] **Internationalization:** Define message catalogs (default en-US), add locale switcher, and wire locale negotiation/fallback early in the UI/API.
- [x] **Themes:** Establish multiple UI themes (e.g., light/high-contrast/dark) with a toggle and persisted preference.
- [ ] **Telemetry Collector:** Optionally ship otelcol/Alloy to receive OTLP and forward traces/logs/metrics; configure endpoints in Settings.

### 2.4 Lifecycle & Safe Updates

- [ ] **Boot Environments (BE):** Wrapper for `beadm list`, `beadm create`, `beadm activate`.
- [ ] **Safe Update Logic (PkgActor):**
  1. Clone current BE.
  2. Mount new BE.
  3. Run `pkg -R <altroot> update`.
  4. Run `bootadm update-archive -R <altroot>`.
  5. Activate new BE.
- [ ] **UI:** "Update System" button streaming logs from the Actor.
- [ ] **Power:** Reboot/Shutdown endpoints.

### 2.5 Network Manager

- [x] **Read:** Parse `ipadm show-addr` for interface listing.
- [x] **Read:** Parse `dladm show-phys` for physical link info (speed, MAC, MTU).
- [x] **Read:** Parse `/etc/resolv.conf` for DNS servers and search domains.
- [x] **Read:** Parse `/etc/defaultrouter` for gateway.
- [x] **Write (NetworkActor):** Set static IP, enable DHCP, update DNS, update gateway.
- [x] **UI:** Network configuration page with per-interface editing.
- [ ] **Write:** MTU configuration.
- [ ] **Future:** VLAN and bond/aggregation support.

---

## Phase 3: The Storage Core (ZFS)

*The core engine. High concurrency safety required.*

### 3.1 Architecture: ZFS Actor

- [x] **Actor Setup:** `ZfsActor` receiving messages (`ListPools`, `GetPool`, `ListDatasets`, `GetDataset`).
- [x] **Serialization:** Ensure ZFS commands are processed sequentially to avoid race conditions.
- [ ] **Polling:** Background task to periodically poll `zpool status` and detect external CLI changes.
- [x] **FFI:** Direct `libzfs` FFI bindings for pool/dataset operations (no CLI parsing).

### 3.2 Observability: DTrace (USDT)

- [ ] **Hooks:** Integrate `usdt` crate.
- [ ] **Probes:** Insert `dtrace_probe!` before/after expensive ZFS ops.

### 3.3 Hardware & Disks

- [ ] **Discovery:** List disks (`format`, `sata`, `nvme`).
- [ ] **SMART:** Wrapper for `smartctl` (JSON output). UI Health Badges.

### 3.4 Pool Management

- [x] **Read:** List pools and get pool info via libzfs FFI.
- [ ] **Write:** Wizard for Stripe, Mirror, RAIDZ1/2/3.
- [ ] **Actions:** Scrub, Export, Import.

### 3.5 Dataset Management

- [x] **Read:** List datasets and get dataset info via libzfs FFI.
- [ ] **Hierarchy:** Tree view of datasets in UI.
- [ ] **Properties:** Compression, Quota, Mountpoints (write).

---

## Phase 4: Sharing & Services

*NAS functionality. Idempotent configuration.*

### 4.1 SMB (Kernel)

- [ ] **Config:** Manage `smbadm` and `sharesmb` property.
- [ ] **ACLs:** UI for ZFS/SMB ACL management.
- [ ] **Idempotency:** Ensure applying settings doesn't break existing connections if config hasn't changed.

### 4.2 NFS

- [ ] **Config:** Manage `sharenfs` property or `/etc/exports`.
- [ ] **Access:** IP-based Host Allow/Deny.

### 4.3 Users & Groups

- [ ] **Local Users:** Wrapper for `useradd`/`passwd` (Shadow DB for share access).
- [ ] **Groups:** Group management.

### 4.4 Service Control

- [ ] **SMF Interface:** Wrapper for `svcs` (status) and `svcadm` (restart/enable).

---

## Phase 5: Data Safety

*Backups and Replication.*

### 5.1 Snapshots

- [ ] **UI:** List, Rollback, Delete, Clone.
- [ ] **Time Slider:** Visual component for snapshot browsing.

### 5.2 Replication

- [ ] **Transport:** SSH Key management.
- [ ] **Engine:** Wrapper for `zfs send | ssh recv`.
- [ ] **Scheduler:** Cron job generator.

### 5.3 Cloud Backup (Restic)

- [ ] **Integration:** Download/Install `restic`.
- [ ] **UI:** Repo Init (S3/SFTP), Backup, Restore browser.

---

## Phase 6: Productization

*Distribution and Polish.*

### 6.1 Custom ISO / Installer

- [ ] **Build Script:** Remaster OmniOS ISO.
- [ ] **Injection:** Embed binary and SMF manifest (`/opt/storage-os`).
- [ ] **Boot Script:** First-boot logic (Certificate generation, Initial Network Setup).

### 6.2 Repo

- [ ] Build a ansible/opentofu role to set up a pkg registry server
- [ ] package yanOS for easy upgrades

### 6.3 Final Polish

- [ ] **Security Audit:** Verify permissions, port exposure, and Cookie attributes.
- [ ] **UX Polish:** Dark Mode, Mobile Responsiveness, Loading States.
- [ ] **Documentation:** README, API Docs, User Guide.
- [ ] **HA Readiness:** Keep API stateless and reconciliation idempotent to allow future active/passive clustering on shared storage.

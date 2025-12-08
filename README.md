# Modern Storage OS

## 1. Vision & Core Philosophy

We are building a **Storage Appliance**, not just a "tool". The user downloads an ISO, installs it on bare metal, and controls everything through a web interface.

**Core Principles:**

*   **System Files = Single Source of Truth:** We use `/etc/` and ZFS Properties. No parallel database.
*   **Production Ready:** Focus on Observability (OTel/DTrace), updates, and security.
*   **Modern DX:** API-First with auto-documentation (OpenAPI) and a Svelte 5 Frontend.

## 2. Technology Stack

*   **OS Base:** OmniOS Community Edition.
*   **Backend:** Rust
    *   **Web:** Axum
    *   **Runtime:** Tokio
    *   **API Docs:** `utoipa`
    *   **Tracing:** `tracing-opentelemetry` + `usdt` (DTrace)
*   **Frontend:** Svelte 5 (Runes) + Tailwind CSS.
    *   **Web Shell:** Integration of `ttyd` via WebSocket proxy directly in the dashboard.
*   **Build System:** `just` (as a modern Make replacement).

## 3. Service Management

*   **Service User:** `webservd` (least privilege for the Axum binary).
*   **SMF Manifest:** `backend/smf/zos-backend.xml` imports the service under `site/storage-os`.

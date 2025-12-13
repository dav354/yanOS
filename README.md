# yanOS

Yet another NAS Os

## 1. Vision & Philosophy

yanOS is a storage-first appliance for OmniOS—leaner than TrueNAS/OMV/Unraid because it avoids Docker/VM sprawl and keeps config in system files.

*   **KISS:** No container zoo; only storage and core services.
*   **Single Source of Truth:** `/etc/` + ZFS properties, never a shadow database.
*   **Production Ready:** Observability (OTel/DTrace), safe updates, strong defaults.
*   **Modern DX:** API-first with generated docs (OpenAPI/utoipa) and a Svelte 5 UI.

## 2. Technology Stack

*   **OS Base:** OmniOS Community Edition.
*   **Backend:** Rust
    *   **Web:** Axum
    *   **Runtime:** Tokio
    *   **API Docs:** `utoipa`
    *   **Tracing:** `tracing-opentelemetry` + `usdt` (DTrace)
*   **Frontend:** Svelte 5 (Runes) + Tailwind CSS.
    *   **Web Shell:** Integration of `ttyd` via WebSocket proxy directly in the dashboard.
*   **Build System:** `just`.

# yanOS (Yet Another NAS OS)

yanOS is an open-source storage appliance that bridges enterprise-grade Unix stability with modern web architecture. Built on **OmniOS (illumos)** and **ZFS**, it combines a memory-safe **Rust** backend with a reactive **Svelte 5** frontend.

Like its phonetic namesake *Janus*—the Roman god of gateways and transitions—yanOS stands at the boundary between data ingress and egress. It acts as a secure, transparent guardian of storage operations, enabling seamless management **without abstracting the system behind a proprietary configuration database**.

---

## 1. Vision & Philosophy

yanOS is a **storage-first appliance for OmniOS**. It is deliberately leaner than TrueNAS, OMV, Unraid, or Napp-It by avoiding Docker/VM sprawl and by keeping configuration in native system files.

* **KISS:** No container zoo—only storage and essential services.
* **Solid foundation:** OmniOS/illumos with native ZFS, SMF-managed services, and Solaris-grade reliability.
* **Single Source of Truth:** Configuration lives in `/etc` and ZFS properties—never in a shadow database.
* **Production-ready:** Built-in observability (OpenTelemetry, DTrace), safe update mechanisms, and strong defaults.
* **Modern DX:** API-first design with generated documentation (OpenAPI via `utoipa`) and a Svelte 5 UI.

---

## 2. Technology Stack

* **OS Base:** OmniOS Community Edition
* **Backend:** Rust

  * **Web Framework:** Axum
  * **Async Runtime:** Tokio
  * **API Documentation:** `utoipa`
  * **Tracing & Observability:** `tracing-opentelemetry` + `usdt` (DTrace)
* **Frontend:** Svelte 5 (Runes) with Tailwind CSS

  * **Web Shell:** `ttyd` integrated via a WebSocket proxy directly into the dashboard
* **Build System:** `just`

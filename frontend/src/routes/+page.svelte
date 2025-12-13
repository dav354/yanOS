<script>
    /**
     * Dashboard page - main system overview with real-time metrics.
     *
     * Connects to /api/v1/metrics/live WebSocket for live system stats.
     * Displays:
     * - System info (hostname, kernel, uptime)
     * - Summary cards (CPU, Memory, ARC, Network)
     * - Time-series charts for CPU, Memory, and Network
     * - Per-core CPU breakdown
     *
     * Auto-reconnects on WebSocket disconnect with exponential backoff.
     */
    import { auth } from '$lib/auth.svelte.js';
    import MetricGraph from '$lib/components/MetricGraph.svelte';

    // --- State ---
    let systemInfo = $state(null);
    let metricsSocket = null;

    // Rolling buffer size: ~3 minutes of 1Hz samples
    const maxPoints = 180;
    let labels = $state([]);

    let cpuData = $state([]);
    let ramUsed = $state([]);
    let ramArc = $state([]);
    let ramFree = $state([]);
    let netRx = $state([]);
    let netTx = $state([]);
    let perCore = $state([]);

    // Update Logic
    let isConnected = $state(false);
    let reconnectAttempts = $state(0);
    let reconnectTimeout = $state(null);
    const MAX_RECONNECT_DELAY = 30000;

    let latestCpu = $derived(cpuData.at(-1) ?? 0);
    let latestRam = $derived(ramUsed.at(-1) ?? 0);
    let latestArc = $derived(ramArc.at(-1) ?? 0);
    let latestFree = $derived(ramFree.at(-1) ?? 0);
    let latestNetRx = $derived(netRx.at(-1) ?? 0);
    let latestNetTx = $derived(netTx.at(-1) ?? 0);

    async function fetchSystemInfo() {
        if (!auth.isAuthenticated) return;
        try {
            const res = await fetch('/api/v1/system/info');
            if (res.ok) {
                systemInfo = await res.json();
            }
        } catch (e) {
            console.error('Failed to load system info', e);
        }
    }

    function formatBytes(bytes) {
        if (bytes < 0 || Number.isNaN(bytes)) return '0 B';
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KiB', 'MiB', 'GiB', 'TiB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

    function formatPercent(val) {
        return `${val.toFixed(1)}%`;
    }

    /**
     * Apply a single metric point to the rolling data buffers.
     * Uses immutable updates to trigger Svelte 5 reactivity.
     */
    function applyMetric(metric) {
        const now = new Date(metric.ts ?? Date.now());
        const timeLabel = `${now.getHours()}:${now.getMinutes().toString().padStart(2, '0')}:${now.getSeconds().toString().padStart(2, '0')}`;

        // CPU data
        const cpuVal = Number(metric.cpu_user ?? 0) + Number(metric.cpu_system ?? 0);

        // Memory data
        const total = Number(metric.memory_total ?? 0);
        const arc = Math.max(0, Number(metric.zfs_arc ?? 0));
        const usedRaw = Math.max(0, Number(metric.memory_used ?? 0));
        const usedWithoutArc = Math.max(0, usedRaw - arc);
        const free = Math.max(0, total - usedRaw);

        // Network data
        const rx = Math.max(0, Number(metric.rx_bytes ?? 0));
        const tx = Math.max(0, Number(metric.tx_bytes ?? 0));

        // Per-core data
        perCore = Array.isArray(metric.per_core) ? [...metric.per_core] : [];

        // Immutable array updates for Svelte 5 reactivity
        // Slice from index 1 if over limit to drop oldest, then add new value
        const slice = (arr, val) => {
            const newArr = arr.length >= maxPoints ? arr.slice(1) : [...arr];
            newArr.push(val);
            return newArr;
        };

        labels = slice(labels, timeLabel);
        cpuData = slice(cpuData, cpuVal);
        ramUsed = slice(ramUsed, usedWithoutArc);
        ramArc = slice(ramArc, arc);
        ramFree = slice(ramFree, free);
        netRx = slice(netRx, rx);
        netTx = slice(netTx, tx);
    }

    function scheduleReconnect() {
        if (reconnectTimeout) return;

        // Exponential backoff: 1s, 2s, 4s, 8s... up to MAX_RECONNECT_DELAY
        const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), MAX_RECONNECT_DELAY);
        console.log(`Scheduling WebSocket reconnect in ${delay}ms (attempt ${reconnectAttempts + 1})`);

        reconnectTimeout = setTimeout(() => {
            reconnectTimeout = null;
            reconnectAttempts++;
            connectMetrics();
        }, delay);
    }

    function connectMetrics() {
        if (!auth.isAuthenticated) {
            if (metricsSocket) {
                metricsSocket.close();
                metricsSocket = null;
            }
            isConnected = false;
            return;
        }

        if (metricsSocket && metricsSocket.readyState <= 1) return;

        const protocol = location.protocol === 'https:' ? 'wss' : 'ws';
        const ws = new WebSocket(`${protocol}://${location.host}/api/v1/metrics/live`);
        metricsSocket = ws;

        ws.onopen = () => {
            console.log('Metrics WebSocket connected');
            isConnected = true;
            reconnectAttempts = 0; // Reset on successful connection
        };

        ws.onerror = (e) => {
            console.error('Metrics WebSocket error', e);
            isConnected = false;
        };

        ws.onmessage = (evt) => {
            try {
                const parsed = JSON.parse(evt.data);

                if (Array.isArray(parsed)) {
                    // Initial batch - apply each metric (triggers reactivity)
                    parsed.forEach(applyMetric);
                    return;
                }

                // Single metric update
                applyMetric(parsed);
            } catch (e) {
                console.error('Failed to parse metrics', e);
            }
        };
        ws.onclose = (e) => {
            console.log('Metrics WebSocket closed', e.code, e.reason);
            metricsSocket = null;
            isConnected = false;

            // Auto-reconnect if still authenticated
            if (auth.isAuthenticated) {
                scheduleReconnect();
            }
        };
    }

    $effect(() => {
        if (auth.isAuthenticated) {
            fetchSystemInfo();
            connectMetrics();
        }
        return () => {
            if (reconnectTimeout) {
                clearTimeout(reconnectTimeout);
                reconnectTimeout = null;
            }
            if (metricsSocket) {
                metricsSocket.close();
                metricsSocket = null;
            }
        };
    });
</script>

<div class="max-w-6xl mx-auto space-y-6">
    <div class="flex justify-between items-center">
        <div>
            <p class="text-sm text-text-muted">Systems Overview</p>
            <h1 class="text-3xl font-bold text-text-main">Dashboard</h1>
        </div>
    </div>

    {#if systemInfo}
        <div class="bg-bg-card shadow rounded-lg p-4 grid grid-cols-2 md:grid-cols-4 gap-4 text-sm border border-border-main">
            <div>
                <span class="block text-text-muted">Hostname</span>
                <span class="font-bold text-text-main">{systemInfo.hostname}</span>
            </div>
            <div>
                <span class="block text-text-muted">Kernel</span>
                <span class="font-bold text-text-main">{systemInfo.kernel_version}</span>
            </div>
            <div>
                <span class="block text-text-muted">Uptime</span>
                <span class="font-bold text-text-main">{systemInfo.uptime}</span>
            </div>
            <div>
                 <span class="block text-text-muted">Status</span>
                 <span class="text-green-600 font-bold">● Online</span>
            </div>
        </div>
    {/if}

    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <div class="bg-bg-card border border-border-main rounded-lg p-4 shadow">
            <p class="text-xs uppercase tracking-wide text-text-muted">CPU</p>
            <p class="text-2xl font-semibold text-text-main">{formatPercent(latestCpu)}</p>
            <p class="text-xs text-text-muted">User + System</p>
        </div>
        <div class="bg-bg-card border border-border-main rounded-lg p-4 shadow">
            <p class="text-xs uppercase tracking-wide text-text-muted">Memory</p>
            <p class="text-2xl font-semibold text-text-main">{formatBytes(latestRam + latestArc)}</p>
            <p class="text-xs text-text-muted">Free: {formatBytes(latestFree)}</p>
        </div>
        <div class="bg-bg-card border border-border-main rounded-lg p-4 shadow">
            <p class="text-xs uppercase tracking-wide text-text-muted">ZFS ARC</p>
            <p class="text-2xl font-semibold text-text-main">{formatBytes(latestArc)}</p>
            <p class="text-xs text-text-muted">Cache footprint</p>
        </div>
        <div class="bg-bg-card border border-border-main rounded-lg p-4 shadow">
            <p class="text-xs uppercase tracking-wide text-text-muted">Network</p>
            <p class="text-2xl font-semibold text-text-main">{formatBytes(latestNetRx)}/s</p>
            <p class="text-xs text-text-muted">TX: {formatBytes(latestNetTx)}/s</p>
        </div>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <MetricGraph 
            title="CPU Usage (%)"
            labels={labels}
            yMin={0}
            yMax={100}
            formatValue={formatPercent}
            datasets={[
                { label: 'CPU Usage', data: cpuData, color: '#2563eb', fill: true }
            ]}
        />

        <MetricGraph 
            title="Memory (used / ARC / free)"
            labels={labels}
            stacked={true}
            formatValue={formatBytes}
            datasets={[
                { label: 'Used (excl. ARC)', data: ramUsed, color: '#7c3aed', fill: true, stack: 'mem' },
                { label: 'ZFS ARC', data: ramArc, color: '#059669', fill: true, stack: 'mem' },
                { label: 'Free', data: ramFree, color: '#9ca3af', fill: true, stack: 'mem' }
            ]}
        />

        <MetricGraph 
            title="Network Traffic"
            labels={labels}
            formatValue={(v) => `${formatBytes(v)}/s`}
            datasets={[
                { label: 'RX (In)', data: netRx, color: '#16a34a', fill: false },
                { label: 'TX (Out)', data: netTx, color: '#2563eb', fill: false }
            ]}
        />

        <div class="bg-bg-card p-4 rounded shadow border border-border-main flex flex-col h-64 items-start justify-center text-text-muted">
            <span class="text-lg font-bold text-text-main mb-2">Storage / IOPS</span>
            <span class="text-sm">Hook metrics actor once ZFS polling lands.</span>
        </div>
    </div>

    <div class="bg-bg-card border border-border-main rounded-lg p-4 shadow">
        <p class="text-xs uppercase tracking-wide text-text-muted">Per-core CPU</p>
        {#if perCore.length}
            <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-3 mt-3">
                {#each perCore as core (core.id)}
                    <div class="border border-border-main rounded p-2 flex flex-col gap-1">
                        <span class="text-xs text-text-muted">CPU{core.id}</span>
                        <span class="text-lg font-semibold text-text-main">
                            {formatPercent(Number(core.cpu_user ?? 0) + Number(core.cpu_system ?? 0))}
                        </span>
                        <span class="text-xs text-text-muted">Idle {formatPercent(Number(core.cpu_idle ?? 0))}</span>
                    </div>
                {/each}
            </div>
        {:else}
            <p class="text-sm text-text-muted mt-2">Waiting for samples...</p>
        {/if}
    </div>
</div>

<script>
    /**
     * Dashboard page - main system overview with real-time metrics.
     *
     * Connects to /api/v1/metrics/live WebSocket for live system stats.
     * Displays:
     * - System info header (hostname, kernel, uptime)
     * - Timeframe selector for chart history
     * - Time-series charts for CPU, Memory, Network, and Storage
     * - Per-core CPU bar chart grouped by physical cores
     *
     * Auto-reconnects on WebSocket disconnect with exponential backoff.
     */
    import { auth } from '$lib/auth.svelte.js';
    import MetricGraph from '$lib/components/MetricGraph.svelte';

    // --- Timeframe Configuration ---
    const timeframes = [
        { label: '1 min', value: 60 },
        { label: '3 min', value: 180 },
        { label: '5 min', value: 300 },
        { label: '10 min', value: 600 },
        { label: '1 hr', value: 3600 },
        { label: '3 hr', value: 10800 },
        { label: '6 hr', value: 21600 },
        { label: '12 hr', value: 43200 },
        { label: '24 hr', value: 86400 },
        { label: '3 days', value: 259200 },
    ];
    let selectedTimeframe = $state(180); // Default 3 min

    // --- State ---
    let systemInfo = $state(null);
    let metricsSocket = null;
    let pools = $state([]);

    // Max buffer: 3 days of 1Hz samples
    const maxBuffer = 259200;
    let allLabels = $state([]);
    let allCpuData = $state([]);
    let allRamUsed = $state([]);
    let allRamArc = $state([]);
    let allRamFree = $state([]);
    let allNetRx = $state([]);
    let allNetTx = $state([]);
    let perCore = $state([]);
    let memoryTotal = $state(0);

    // Derived: slice data based on selected timeframe
    let labels = $derived(allLabels.slice(-selectedTimeframe));
    let cpuData = $derived(allCpuData.slice(-selectedTimeframe));
    let ramUsed = $derived(allRamUsed.slice(-selectedTimeframe));
    let ramArc = $derived(allRamArc.slice(-selectedTimeframe));
    let ramFree = $derived(allRamFree.slice(-selectedTimeframe));
    let netRx = $derived(allNetRx.slice(-selectedTimeframe));
    let netTx = $derived(allNetTx.slice(-selectedTimeframe));

    // Update Logic
    let isConnected = $state(false);
    let reconnectAttempts = $state(0);
    let reconnectTimeout = $state(null);
    const MAX_RECONNECT_DELAY = 30000;

    // Group cores by physical core (pairs of hyperthreads)
    let groupedCores = $derived.by(() => {
        if (!perCore.length) return [];
        const groups = [];
        for (let i = 0; i < perCore.length; i += 2) {
            const core1 = perCore[i];
            const core2 = perCore[i + 1];
            const usage1 = Number(core1?.cpu_user ?? 0) + Number(core1?.cpu_system ?? 0);
            const usage2 = core2 ? Number(core2?.cpu_user ?? 0) + Number(core2?.cpu_system ?? 0) : 0;
            groups.push({
                id: Math.floor(i / 2),
                thread1: { id: core1?.id ?? i, usage: usage1 },
                thread2: core2 ? { id: core2.id, usage: usage2 } : null,
                avgUsage: core2 ? (usage1 + usage2) / 2 : usage1
            });
        }
        return groups;
    });

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

    async function fetchPools() {
        if (!auth.isAuthenticated) return;
        try {
            const res = await fetch('/api/v1/storage/pools');
            if (res.ok) {
                pools = await res.json();
            }
        } catch (e) {
            console.error('Failed to load pools', e);
        }
    }

    function getHealthColor(health) {
        switch (health?.toUpperCase()) {
            case 'ONLINE': return 'text-green-500';
            case 'DEGRADED': return 'text-yellow-500';
            case 'FAULTED':
            case 'OFFLINE':
            case 'UNAVAIL': return 'text-red-500';
            default: return 'text-text-muted';
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

    function formatUptime(seconds) {
        if (!seconds || seconds < 0) return 'Unknown';
        const days = Math.floor(seconds / 86400);
        const hours = Math.floor((seconds % 86400) / 3600);
        const mins = Math.floor((seconds % 3600) / 60);

        const parts = [];
        if (days > 0) parts.push(`${days}d`);
        if (hours > 0) parts.push(`${hours}h`);
        if (mins > 0 || parts.length === 0) parts.push(`${mins}m`);
        return parts.join(' ');
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
        if (total > 0) memoryTotal = total;
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
        const slice = (arr, val) => {
            const newArr = arr.length >= maxBuffer ? arr.slice(1) : [...arr];
            newArr.push(val);
            return newArr;
        };

        allLabels = slice(allLabels, timeLabel);
        allCpuData = slice(allCpuData, cpuVal);
        allRamUsed = slice(allRamUsed, usedWithoutArc);
        allRamArc = slice(allRamArc, arc);
        allRamFree = slice(allRamFree, free);
        allNetRx = slice(allNetRx, rx);
        allNetTx = slice(allNetTx, tx);
    }

    function scheduleReconnect() {
        if (reconnectTimeout) return;

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
            reconnectAttempts = 0;
        };

        ws.onerror = (e) => {
            console.error('Metrics WebSocket error', e);
            isConnected = false;
        };

        ws.onmessage = (evt) => {
            try {
                const parsed = JSON.parse(evt.data);

                if (Array.isArray(parsed)) {
                    parsed.forEach(applyMetric);
                    return;
                }

                applyMetric(parsed);
            } catch (e) {
                console.error('Failed to parse metrics', e);
            }
        };
        ws.onclose = (e) => {
            console.log('Metrics WebSocket closed', e.code, e.reason);
            metricsSocket = null;
            isConnected = false;

            if (auth.isAuthenticated) {
                scheduleReconnect();
            }
        };
    }

    $effect(() => {
        if (auth.isAuthenticated) {
            fetchSystemInfo();
            fetchPools();
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
    <!-- Header with system info and timeframe selector -->
    <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
            <h1 class="text-2xl font-bold text-text-main">
                {systemInfo?.hostname ?? 'Dashboard'}
            </h1>
            {#if systemInfo}
                <p class="text-sm text-text-muted mt-1">
                    {systemInfo.kernel_version}
                    <span class="mx-2 text-border-main">|</span>
                    <span class="text-text-main font-medium">{formatUptime(systemInfo.uptime)}</span> uptime
                </p>
            {/if}
        </div>

        <div class="flex items-center gap-2">
            <span class="text-sm text-text-muted">Timeframe:</span>
            <select
                bind:value={selectedTimeframe}
                class="bg-bg-card border border-border-main rounded px-3 py-1.5 text-sm text-text-main focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
                {#each timeframes as tf}
                    <option value={tf.value}>{tf.label}</option>
                {/each}
            </select>
        </div>
    </div>

    <!-- Main charts grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <MetricGraph
            title="CPU Usage"
            labels={labels}
            yMin={0}
            yMax={100}
            formatValue={formatPercent}
            datasets={[
                { label: 'CPU %', data: cpuData, color: '#2563eb', fill: true }
            ]}
        />

        <MetricGraph
            title="Memory"
            labels={labels}
            stacked={true}
            formatValue={formatBytes}
            datasets={[
                { label: 'Used', data: ramUsed, color: '#7c3aed', fill: true, stack: 'mem' },
                { label: 'ARC', data: ramArc, color: '#059669', fill: true, stack: 'mem' },
                { label: 'Free', data: ramFree, color: '#9ca3af', fill: true, stack: 'mem' }
            ]}
        />

        <MetricGraph
            title="Network I/O"
            labels={labels}
            formatValue={(v) => `${formatBytes(v)}/s`}
            datasets={[
                { label: 'RX', data: netRx, color: '#16a34a', fill: false },
                { label: 'TX', data: netTx, color: '#2563eb', fill: false }
            ]}
        />

        <!-- Storage Pools widget -->
        <div class="bg-bg-card p-4 rounded shadow border border-border-main h-64 overflow-auto">
            <div class="flex items-center justify-between mb-3">
                <h3 class="text-sm font-bold text-text-muted uppercase tracking-wide">Storage Pools</h3>
                <a href="/storage" class="text-xs text-primary hover:underline">View all</a>
            </div>
            {#if pools.length === 0}
                <div class="flex items-center justify-center h-40 text-text-muted text-sm">
                    No pools found
                </div>
            {:else}
                <div class="space-y-3">
                    {#each pools as pool (pool.name)}
                        <div class="border border-border-main rounded p-2">
                            <div class="flex items-center justify-between mb-1">
                                <span class="font-medium text-text-main text-sm">{pool.name}</span>
                                <span class="text-xs font-medium {getHealthColor(pool.health)}">{pool.health}</span>
                            </div>
                            <div class="h-1.5 bg-bg-main rounded-full overflow-hidden">
                                <div
                                    class="h-full transition-all"
                                    class:bg-green-500={pool.capacity < 70}
                                    class:bg-yellow-500={pool.capacity >= 70 && pool.capacity < 85}
                                    class:bg-red-500={pool.capacity >= 85}
                                    style="width: {Math.min(100, pool.capacity)}%"
                                ></div>
                            </div>
                            <div class="flex justify-between mt-1 text-xs text-text-muted">
                                <span>{pool.capacity}% used</span>
                                <span>{formatBytes(pool.free)} free</span>
                            </div>
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
    </div>

    <!-- Per-core CPU vertical bar chart -->
    <div class="bg-bg-card border border-border-main rounded-lg p-4 shadow">
        <h3 class="text-sm font-bold text-text-muted uppercase tracking-wide mb-4">CPU Cores</h3>

        {#if groupedCores.length}
            <div class="flex">
                <!-- Y-axis scale -->
                <div class="flex flex-col justify-between h-24 pr-2 text-xs text-text-muted">
                    <span>100%</span>
                    <span>50%</span>
                    <span>0%</span>
                </div>
                <!-- Bars -->
                <div class="flex items-end justify-between flex-1 h-24">
                    {#each groupedCores as group, idx (idx)}
                        <div class="flex flex-col items-center gap-0.5 flex-1">
                            <div class="flex gap-px h-24 items-end w-full justify-center">
                                <div
                                    class="flex-1 max-w-3 bg-blue-500 rounded-t transition-all duration-300"
                                    style="height: {Math.min(100, group.thread1.usage)}%"
                                    title="Core {group.id} T0: {formatPercent(group.thread1.usage)}"
                                ></div>
                                {#if group.thread2}
                                    <div
                                        class="flex-1 max-w-3 bg-indigo-500 rounded-t transition-all duration-300"
                                        style="height: {Math.min(100, group.thread2.usage)}%"
                                        title="Core {group.id} T1: {formatPercent(group.thread2.usage)}"
                                    ></div>
                                {/if}
                            </div>
                            <span class="text-xs text-text-muted">{group.id}</span>
                        </div>
                    {/each}
                </div>
            </div>
        {:else}
            <p class="text-sm text-text-muted">Waiting for data...</p>
        {/if}
    </div>
</div>

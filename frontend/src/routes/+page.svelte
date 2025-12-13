<script>
    import { auth } from '$lib/auth.svelte.js';
    import MetricGraph from '$lib/components/MetricGraph.svelte';

    let systemInfo = $state(null);
    let metricsSocket = null;

    const maxPoints = 180; // keep ~3 minutes at 1Hz
    let labels = $state([]);

    let cpuData = $state([]);
    let ramUsed = $state([]);
    let ramArc = $state([]);
    let ramFree = $state([]);
    let netRx = $state([]);
    let netTx = $state([]);

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

    function applyMetric(metric) {
        const now = new Date(metric.ts ?? Date.now());
        const timeLabel = `${now.getHours()}:${now.getMinutes().toString().padStart(2, '0')}:${now.getSeconds().toString().padStart(2, '0')}`;

        labels.push(timeLabel);
        if (labels.length > maxPoints) labels.shift();

        const cpuVal = Number(metric.cpu_user ?? 0) + Number(metric.cpu_system ?? 0);
        cpuData.push(cpuVal);
        if (cpuData.length > maxPoints) cpuData.shift();

        const total = Number(metric.memory_total ?? 0);
        const arc = Math.max(0, Number(metric.zfs_arc ?? 0));
        const usedRaw = Math.max(0, Number(metric.memory_used ?? 0));
        const usedWithoutArc = Math.max(0, usedRaw - arc);
        const free = Math.max(0, total - usedRaw);

        ramUsed.push(usedWithoutArc);
        ramArc.push(arc);
        ramFree.push(free);
        if (ramUsed.length > maxPoints) {
            ramUsed.shift();
            ramArc.shift();
            ramFree.shift();
        }

        netRx.push(Math.max(0, Number(metric.rx_bytes ?? 0)));
        netTx.push(Math.max(0, Number(metric.tx_bytes ?? 0)));
        if (netRx.length > maxPoints) {
            netRx.shift();
            netTx.shift();
        }
    }

    function triggerUpdate() {
        labels = [...labels];
        cpuData = [...cpuData];
        ramUsed = [...ramUsed];
        ramArc = [...ramArc];
        ramFree = [...ramFree];
        netRx = [...netRx];
        netTx = [...netTx];
    }

    function connectMetrics() {
        if (!auth.isAuthenticated) {
            if (metricsSocket) {
                metricsSocket.close();
                metricsSocket = null;
            }
            return;
        }
        
        if (metricsSocket && metricsSocket.readyState <= 1) return;

        const protocol = location.protocol === 'https:' ? 'wss' : 'ws';
        const ws = new WebSocket(`${protocol}://${location.host}/api/v1/metrics/live`);
        metricsSocket = ws;
        
        ws.onopen = () => {
            console.log('Metrics WebSocket connected');
        };

        ws.onerror = (e) => {
            console.error('Metrics WebSocket error', e);
        };

        let batchTimer = null;
        
        ws.onmessage = (evt) => {
            // console.debug('Metrics received', evt.data.length);
            try {
                const parsed = JSON.parse(evt.data);

                if (Array.isArray(parsed)) {
                    console.log('Received history batch', parsed.length);
                    parsed.forEach(applyMetric);
                    triggerUpdate();
                    return;
                }

                applyMetric(parsed);

                if (!batchTimer) {
                    batchTimer = setTimeout(() => {
                        triggerUpdate();
                        batchTimer = null;
                    }, 60);
                }
            } catch (e) {
                console.error('Failed to parse metrics', e);
            }
        };
        ws.onclose = (e) => {
            console.log('Metrics WebSocket closed', e.code, e.reason);
            metricsSocket = null;
        };
    }

    $effect(() => {
        if (auth.isAuthenticated) {
            fetchSystemInfo();
            connectMetrics();
        }
        return () => {
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
        <div class="text-xs text-text-muted">Live 1s interval</div>
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
</div>

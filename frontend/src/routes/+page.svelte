<script>
    import { onMount } from 'svelte';
    import { auth } from '$lib/auth.svelte.js';

    let username = $state('');
    let password = $state('');
    let systemInfo = $state(null);
    let events = $state([]);
    let metrics = $state(null);
    let interfaces = $state([]);
    let packages = $state([]);
    let eventSocket = null;
    let metricsSocket = null;

    async function handleLogin(event) {
        event.preventDefault();
        await auth.login(username, password);
        await fetchSystemInfo();
        await fetchNetwork();
        await fetchPackages();
        connectMetrics();
    }

    async function fetchSystemInfo() {
        try {
            const res = await fetch('/api/v1/system/info');
            if (res.ok) {
                systemInfo = await res.json();
            }
        } catch (e) {
            console.error('Failed to load system info', e);
        }
    }

    async function fetchNetwork() {
        if (!auth.isAuthenticated) return;
        try {
            const res = await fetch('/api/v1/network/interfaces');
            if (res.ok) {
                interfaces = await res.json();
            }
        } catch (e) {
            console.error('Failed to load interfaces', e);
        }
    }

    async function fetchPackages() {
        if (!auth.isAuthenticated) return;
        try {
            const res = await fetch('/api/v1/pkg/list');
            if (res.ok) {
                packages = await res.json();
            }
        } catch (e) {
            console.error('Failed to load packages', e);
        }
    }

    function connectEvents() {
        if (!auth.isAuthenticated) {
            if (eventSocket) {
                eventSocket.close();
                eventSocket = null;
            }
            return;
        }
        const protocol = location.protocol === 'https:' ? 'wss' : 'ws';
        const ws = new WebSocket(`${protocol}://${location.host}/api/v1/events`);
        eventSocket = ws;
        ws.onmessage = (evt) => {
            try {
                const payload = JSON.parse(evt.data);
                events = [{ ...payload, ts: new Date().toISOString() }, ...events].slice(0, 20);
            } catch (e) {
                console.error('Failed to parse event', e);
            }
        };
        ws.onclose = () => {
            eventSocket = null;
        };
    }

    function connectMetrics() {
        if (!auth.isAuthenticated) {
            if (metricsSocket) {
                metricsSocket.close();
                metricsSocket = null;
            }
            return;
        }
        const protocol = location.protocol === 'https:' ? 'wss' : 'ws';
        const ws = new WebSocket(`${protocol}://${location.host}/api/v1/metrics/live`);
        metricsSocket = ws;
        ws.onmessage = (evt) => {
            try {
                metrics = JSON.parse(evt.data);
            } catch (e) {
                console.error('Failed to parse metrics', e);
            }
        };
        ws.onclose = () => {
            metricsSocket = null;
        };
    }

    onMount(() => {
        fetchSystemInfo();
    });

    $effect(() => {
        connectEvents();
        connectMetrics();
        fetchNetwork();
        fetchPackages();
        return () => {
            if (eventSocket) {
                eventSocket.close();
                eventSocket = null;
            }
            if (metricsSocket) {
                metricsSocket.close();
                metricsSocket = null;
            }
        };
    });
</script>

<div class="p-4 space-y-6">
    <h1 class="text-2xl font-bold">zOS Management</h1>

    {#if auth.isAuthenticated}
        <div class="bg-green-100 p-4 rounded">
            <p>Welcome, <strong>{auth.user}</strong>!</p>
        </div>
    {:else}
        <form onsubmit={handleLogin} class="bg-gray-100 p-4 rounded max-w-sm space-y-4">
            <div>
                <label class="block text-gray-700 text-sm font-bold mb-2" for="username">
                    Username
                </label>
                <input class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline" id="username" type="text" bind:value={username}>
            </div>
            <div>
                <label class="block text-gray-700 text-sm font-bold mb-2" for="password">
                    Password
                </label>
                <input class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 mb-3 leading-tight focus:outline-none focus:shadow-outline" id="password" type="password" bind:value={password}>
            </div>
            <div class="flex items-center justify-between">
                <button class="bg-blue-600 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline" type="submit">
                    Sign In
                </button>
            </div>
        </form>
    {/if}

    <div class="grid gap-4 md:grid-cols-2">
        <div class="bg-white shadow rounded p-4">
            <h2 class="text-lg font-semibold mb-2">System Info</h2>
            {#if systemInfo}
                <dl class="space-y-1 text-sm">
                    <div class="flex justify-between">
                        <dt class="font-medium text-gray-600">Hostname</dt>
                        <dd class="font-mono text-gray-900">{systemInfo.hostname}</dd>
                    </div>
                    <div class="flex justify-between">
                        <dt class="font-medium text-gray-600">Kernel</dt>
                        <dd class="font-mono text-gray-900">{systemInfo.kernel_version}</dd>
                    </div>
                    <div class="flex justify-between">
                        <dt class="font-medium text-gray-600">Uptime (s)</dt>
                        <dd class="font-mono text-gray-900">{systemInfo.uptime}</dd>
                    </div>
                </dl>
            {:else}
                <p class="text-sm text-gray-500">Loading system info…</p>
            {/if}
        </div>

        <div class="bg-white shadow rounded p-4">
            <h2 class="text-lg font-semibold mb-2">External Events</h2>
            {#if events.length === 0}
                <p class="text-sm text-gray-500">No events yet.</p>
            {:else}
                <ul class="space-y-2 max-h-64 overflow-y-auto text-sm">
                    {#each events as event}
                        <li class="border rounded p-2">
                            <div class="text-gray-700 font-medium">{event.type}</div>
                            {#if event.path}
                                <div class="font-mono text-gray-800 text-xs break-all">{event.path}</div>
                            {/if}
                            <div class="text-gray-500 text-xs">{event.ts}</div>
                        </li>
                    {/each}
                </ul>
            {/if}
        </div>

        <div class="bg-white shadow rounded p-4">
            <h2 class="text-lg font-semibold mb-2">Metrics</h2>
            {#if metrics}
                <dl class="space-y-1 text-sm">
                    <div class="flex justify-between">
                        <dt class="font-medium text-gray-600">CPU User</dt>
                        <dd class="font-mono text-gray-900">{metrics.cpu_user}</dd>
                    </div>
                    <div class="flex justify-between">
                        <dt class="font-medium text-gray-600">CPU Idle</dt>
                        <dd class="font-mono text-gray-900">{metrics.cpu_idle}</dd>
                    </div>
                </dl>
            {:else}
                <p class="text-sm text-gray-500">Waiting for metrics…</p>
            {/if}
        </div>

        <div class="bg-white shadow rounded p-4">
            <h2 class="text-lg font-semibold mb-2">Network Interfaces</h2>
            {#if interfaces.length === 0}
                <p class="text-sm text-gray-500">No interfaces yet.</p>
            {:else}
                <ul class="space-y-1 text-sm">
                    {#each interfaces as iface}
                        <li class="flex justify-between border rounded px-2 py-1">
                            <span class="font-mono">{iface.name}</span>
                            <span class="text-gray-700">{iface.address}</span>
                            <span class="text-gray-500">{iface.state}</span>
                        </li>
                    {/each}
                </ul>
            {/if}
        </div>

        <div class="bg-white shadow rounded p-4 md:col-span-2">
            <h2 class="text-lg font-semibold mb-2">Packages</h2>
            {#if packages.length === 0}
                <p class="text-sm text-gray-500">No packages yet.</p>
            {:else}
                <div class="overflow-x-auto">
                    <table class="min-w-full text-sm">
                        <thead>
                            <tr class="text-left text-gray-600">
                                <th class="py-1 pr-4">Name</th>
                                <th class="py-1 pr-4">Version</th>
                                <th class="py-1 pr-4">Status</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each packages as pkg}
                                <tr class="border-t">
                                    <td class="py-1 pr-4 font-mono">{pkg.name}</td>
                                    <td class="py-1 pr-4 font-mono">{pkg.version}</td>
                                    <td class="py-1 pr-4">{pkg.status}</td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            {/if}
        </div>
    </div>
</div>

<script>
    import { auth } from '$lib/auth.svelte.js';

    let interfaces = $state([]);
    let isLoading = $state(false);

    async function fetchNetwork() {
        if (!auth.isAuthenticated) return;
        isLoading = true;
        try {
            const res = await fetch('/api/v1/network/interfaces');
            if (res.ok) {
                interfaces = await res.json();
            }
        } catch (e) {
            console.error('Failed to load interfaces', e);
        } finally {
            isLoading = false;
        }
    }

    $effect(() => {
        if (auth.isAuthenticated) {
            fetchNetwork();
        }
    });
</script>

<div class="space-y-6">
    <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold text-text-main">Network Interfaces</h1>
        <button onclick={fetchNetwork} class="text-primary hover:text-primary-hover text-sm font-medium">
            ↻ Refresh
        </button>
    </div>

    <div class="bg-bg-card shadow-md rounded-lg overflow-hidden border border-border-main">
        {#if isLoading && interfaces.length === 0}
            <div class="p-8 text-center text-text-muted">Loading interfaces...</div>
        {:else if interfaces.length === 0}
            <div class="p-8 text-center text-text-muted">No interfaces found.</div>
        {:else}
            <table class="min-w-full divide-y divide-border-main">
                <thead class="bg-bg-main">
                    <tr>
                        <th class="px-6 py-3 text-left text-xs font-medium text-text-muted uppercase tracking-wider">Name</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-text-muted uppercase tracking-wider">State</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-text-muted uppercase tracking-wider">IP Address</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-text-muted uppercase tracking-wider">MAC / Details</th>
                    </tr>
                </thead>
                <tbody class="bg-bg-card divide-y divide-border-main">
                    {#each interfaces as iface}
                        <tr class="hover:bg-bg-main transition-colors">
                            <td class="px-6 py-4 whitespace-nowrap text-sm font-medium text-text-main font-mono">
                                {iface.name}
                            </td>
                            <td class="px-6 py-4 whitespace-nowrap text-sm">
                                <span class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full 
                                    {iface.state === 'up' ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}">
                                    {iface.state}
                                </span>
                            </td>
                            <td class="px-6 py-4 whitespace-nowrap text-sm text-text-muted font-mono">
                                {iface.address}
                            </td>
                            <td class="px-6 py-4 whitespace-nowrap text-sm text-text-muted">
                                <!-- Placeholder for more info if available -->
                                <span class="text-xs text-text-muted">Via dladm/ipadm</span>
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        {/if}
    </div>
</div>

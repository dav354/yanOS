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
        <h1 class="text-3xl font-bold text-gray-800">Network Interfaces</h1>
        <button onclick={fetchNetwork} class="text-blue-600 hover:text-blue-800 text-sm font-medium">
            ↻ Refresh
        </button>
    </div>

    <div class="bg-white shadow-md rounded-lg overflow-hidden">
        {#if isLoading && interfaces.length === 0}
            <div class="p-8 text-center text-gray-500">Loading interfaces...</div>
        {:else if interfaces.length === 0}
            <div class="p-8 text-center text-gray-500">No interfaces found.</div>
        {:else}
            <table class="min-w-full divide-y divide-gray-200">
                <thead class="bg-gray-50">
                    <tr>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Name</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">State</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">IP Address</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">MAC / Details</th>
                    </tr>
                </thead>
                <tbody class="bg-white divide-y divide-gray-200">
                    {#each interfaces as iface}
                        <tr class="hover:bg-gray-50">
                            <td class="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900 font-mono">
                                {iface.name}
                            </td>
                            <td class="px-6 py-4 whitespace-nowrap text-sm">
                                <span class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full 
                                    {iface.state === 'up' ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}">
                                    {iface.state}
                                </span>
                            </td>
                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 font-mono">
                                {iface.address}
                            </td>
                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                <!-- Placeholder for more info if available -->
                                <span class="text-xs text-gray-400">Via dladm/ipadm</span>
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        {/if}
    </div>
</div>

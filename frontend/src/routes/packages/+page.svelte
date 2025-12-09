<script>
    import { auth } from '$lib/auth.svelte.js';

    let packages = $state([]);
    let isLoading = $state(false);

    async function fetchPackages() {
        if (!auth.isAuthenticated) return;
        isLoading = true;
        try {
            const res = await fetch('/api/v1/pkg/list');
            if (res.ok) {
                packages = await res.json();
            }
        } catch (e) {
            console.error('Failed to load packages', e);
        } finally {
            isLoading = false;
        }
    }

    $effect(() => {
        if (auth.isAuthenticated) {
            fetchPackages();
        }
    });
</script>

<div class="space-y-6">
    <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold text-gray-800">Installed Packages</h1>
        <button onclick={fetchPackages} class="text-blue-600 hover:text-blue-800 text-sm font-medium">
            ↻ Refresh
        </button>
    </div>

    <div class="bg-white shadow-md rounded-lg overflow-hidden">
        {#if isLoading && packages.length === 0}
            <div class="p-8 text-center text-gray-500">Loading package list...</div>
        {:else if packages.length === 0}
            <div class="p-8 text-center text-gray-500">No packages found (or unable to fetch).</div>
        {:else}
            <div class="overflow-x-auto">
                <table class="min-w-full divide-y divide-gray-200">
                    <thead class="bg-gray-50">
                        <tr>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">FMRI / Name</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Version</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Status</th>
                        </tr>
                    </thead>
                    <tbody class="bg-white divide-y divide-gray-200">
                        {#each packages as pkg}
                            <tr class="hover:bg-gray-50">
                                <td class="px-6 py-2 whitespace-nowrap text-sm font-medium text-gray-900 font-mono">
                                    {pkg.name}
                                </td>
                                <td class="px-6 py-2 whitespace-nowrap text-sm text-gray-500 font-mono">
                                    {pkg.version}
                                </td>
                                <td class="px-6 py-2 whitespace-nowrap text-sm text-gray-500">
                                    {pkg.status}
                                </td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        {/if}
    </div>
</div>

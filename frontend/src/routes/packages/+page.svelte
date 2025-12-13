<script>
    import { auth } from '$lib/auth.svelte.js';
    import { i18n } from '$lib/i18n.svelte.js';

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
        <h1 class="text-3xl font-bold text-text-main">{i18n.t('packages.title')}</h1>
        <button onclick={fetchPackages} class="text-primary hover:text-primary-hover text-sm font-medium">
            ↻ {i18n.t('packages.refresh')}
        </button>
    </div>

    <div class="bg-bg-card shadow-md rounded-lg overflow-hidden border border-border-main">
        {#if isLoading && packages.length === 0}
            <div class="p-8 text-center text-text-muted">{i18n.t('packages.loading')}</div>
        {:else if packages.length === 0}
            <div class="p-8 text-center text-text-muted">{i18n.t('packages.empty')}</div>
        {:else}
            <div class="overflow-x-auto">
                <table class="min-w-full divide-y divide-border-main">
                    <thead class="bg-bg-main">
                        <tr>
                            <th class="px-6 py-3 text-left text-xs font-medium text-text-muted uppercase tracking-wider">Name</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-text-muted uppercase tracking-wider">Version</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-text-muted uppercase tracking-wider">Build Time</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-text-muted uppercase tracking-wider">Status</th>
                        </tr>
                    </thead>
                    <tbody class="bg-bg-card divide-y divide-border-main">
                        {#each packages as pkg}
                            <tr class="hover:bg-bg-main transition-colors">
                                <td class="px-6 py-2 whitespace-nowrap text-sm font-medium text-text-main font-mono">
                                    {pkg.name}
                                </td>
                                <td class="px-6 py-2 whitespace-nowrap text-sm text-text-muted font-mono">
                                    {pkg.version}
                                </td>
                                <td class="px-6 py-2 whitespace-nowrap text-sm text-text-muted font-mono">
                                    {pkg.build_time}
                                </td>
                                <td class="px-6 py-2 whitespace-nowrap text-sm text-text-muted">
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

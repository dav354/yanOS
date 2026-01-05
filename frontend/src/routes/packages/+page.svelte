<script>
    import { auth } from '$lib/auth.svelte.js';
    import { i18n } from '$lib/i18n.svelte.js';

    let packages = $state([]);
    let updates = $state([]);
    let isLoading = $state(false);

    let displayList = $derived.by(() => {
        const updateMap = new Map(updates.map(u => [u.name, u]));
        
        return packages.map(pkg => {
            const update = updateMap.get(pkg.name);
            const has_update = !!update;
            const new_version = update?.version;
            const new_build_time = update?.build_time;

            const show_version_diff = has_update && pkg.version !== new_version;
            // Only show build time diff if version is SAME but build time differs
            const show_build_time_diff = has_update && !show_version_diff && pkg.build_time !== new_build_time;

            return {
                ...pkg,
                has_update,
                new_version,
                new_build_time,
                show_version_diff,
                show_build_time_diff
            };
        }).sort((a, b) => {
            if (a.has_update && !b.has_update) return -1;
            if (!a.has_update && b.has_update) return 1;
            return a.name.localeCompare(b.name);
        });
    });

    let updateCount = $derived(updates.length);

    async function fetchPackages() {
        if (!auth.isAuthenticated) return;
        isLoading = true;
        try {
            const [pkgRes, upRes] = await Promise.all([
                fetch('/api/v1/pkg/list'),
                fetch('/api/v1/pkg/updates')
            ]);

            if (pkgRes.ok) packages = await pkgRes.json();
            if (upRes.ok) updates = await upRes.json();
            
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

    function applyUpdate(name) {
        // Encode command for terminal
        const cmd = encodeURIComponent(name ? `pkg update ${name}` : 'pkg update');
        window.location.href = `/terminal?cmd=${cmd}`;
    }

    async function checkUpdates() {
        if (!auth.isAuthenticated) return;
        try {
            await fetch('/api/v1/pkg/updates/check', { method: 'POST' });
            // Task events will show progress in sidebar
        } catch (e) {
            console.error('Failed to trigger update check', e);
        }
    }

    function formatIPSDate(str) {
        if (!str || str.length < 15) return str;
        // 20251023T162124Z -> 2025-10-23T16:21:24Z
        const iso = `${str.substring(0,4)}-${str.substring(4,6)}-${str.substring(6,8)}T${str.substring(9,11)}:${str.substring(11,13)}:${str.substring(13,15)}Z`;
        try {
            const d = new Date(iso);
            return d.toLocaleString(undefined, { 
                day: '2-digit', month: '2-digit', year: 'numeric', 
                hour: '2-digit', minute: '2-digit' 
            });
        } catch {
            return str;
        }
    }
</script>

<div class="space-y-6">
    <div class="flex items-center justify-between">
        <h1 class="text-2xl font-bold text-text-main">{i18n.t('packages.title')}</h1>
        <div class="flex gap-2">
            <button onclick={checkUpdates} class="text-text-main hover:text-primary px-3 py-2 rounded text-sm font-medium transition-colors border border-border-main hover:border-primary">
                {i18n.t('packages.checkUpdates')}
            </button>
        </div>
    </div>

    {#if updateCount > 0}
        <div class="bg-primary/5 border border-primary/20 rounded-lg p-4 flex items-center justify-between">
            <div class="flex items-center gap-3">
                <div class="text-primary">
                    <!-- Icon placeholder -->
                    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/></svg>
                </div>
                <div>
                    <span class="font-bold text-text-main">{updateCount} {i18n.t('packages.updatesAvailable')}</span>
                    <p class="text-xs text-text-muted mt-0.5">System stability improvements and security fixes may be included.</p>
                </div>
            </div>
            <button 
                onclick={() => applyUpdate(null)}
                class="bg-primary hover:bg-primary-hover text-primary-fg px-4 py-2 rounded text-sm font-bold transition-colors shadow-sm"
            >
                {i18n.t('packages.applyAll')}
            </button>
        </div>
    {/if}

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
                            <th 
                                class="px-6 py-3 text-left text-xs font-medium text-text-muted uppercase tracking-wider cursor-help"
                                title={i18n.t('packages.statusHeader')}
                            >
                                Status ⓘ
                            </th>
                            <th class="px-6 py-3 text-right text-xs font-medium text-text-muted uppercase tracking-wider">Action</th>
                        </tr>
                    </thead>
                    <tbody class="bg-bg-card divide-y divide-border-main">
                        {#each displayList as pkg}
                            <tr class="hover:bg-bg-main transition-colors {pkg.has_update ? 'bg-primary/5' : ''}">
                                <td class="px-6 py-2 whitespace-nowrap text-sm font-medium text-text-main font-mono">
                                    {pkg.name}
                                </td>
                                <td class="px-6 py-2 whitespace-nowrap text-sm text-text-muted font-mono">
                                    {#if pkg.show_version_diff}
                                        <div class="flex flex-col">
                                            <span class="line-through opacity-60">{pkg.version}</span>
                                            <span class="text-primary font-bold">{pkg.new_version}</span>
                                        </div>
                                    {:else}
                                        {pkg.version}
                                    {/if}
                                </td>
                                <td class="px-6 py-2 whitespace-nowrap text-sm text-text-muted font-mono">
                                    {#if pkg.show_build_time_diff}
                                        <div class="flex flex-col">
                                            <span class="line-through opacity-60">{formatIPSDate(pkg.build_time)}</span>
                                            <span class="text-primary font-bold">{formatIPSDate(pkg.new_build_time)}</span>
                                        </div>
                                    {:else}
                                        {formatIPSDate(pkg.build_time)}
                                    {/if}
                                </td>
                                <td class="px-6 py-2 whitespace-nowrap text-sm text-text-muted">
                                    {#if pkg.has_update}
                                        <span class="text-primary font-bold">{i18n.t('packages.updateAvailableStatus')}</span>
                                    {:else}
                                        {pkg.status}
                                    {/if}
                                </td>
                                <td class="px-6 py-2 whitespace-nowrap text-sm text-right">
                                    {#if pkg.has_update}
                                        <button 
                                            onclick={() => applyUpdate(pkg.name)}
                                            class="text-primary hover:text-primary-hover font-bold hover:underline"
                                        >
                                            {i18n.t('packages.updateAction')}
                                        </button>
                                    {/if}
                                </td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        {/if}
    </div>
</div>

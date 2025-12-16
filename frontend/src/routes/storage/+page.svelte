<script>
    /**
     * Storage page - ZFS pool and dataset management.
     *
     * Displays a list of all ZFS pools with their health status,
     * capacity, and basic statistics.
     */
    import { auth } from '$lib/auth.svelte.js';
    import { i18n } from '$lib/i18n.svelte.js';

    let pools = $state([]);
    let isLoading = $state(false);
    let error = $state(null);

    function formatBytes(bytes) {
        if (bytes < 0 || Number.isNaN(bytes)) return '0 B';
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

    function getHealthColor(health) {
        switch (health?.toUpperCase()) {
            case 'ONLINE':
                return 'text-green-500';
            case 'DEGRADED':
                return 'text-yellow-500';
            case 'FAULTED':
            case 'OFFLINE':
            case 'UNAVAIL':
                return 'text-red-500';
            default:
                return 'text-text-muted';
        }
    }

    function getHealthBg(health) {
        switch (health?.toUpperCase()) {
            case 'ONLINE':
                return 'bg-green-500/10 border-green-500/30';
            case 'DEGRADED':
                return 'bg-yellow-500/10 border-yellow-500/30';
            case 'FAULTED':
            case 'OFFLINE':
            case 'UNAVAIL':
                return 'bg-red-500/10 border-red-500/30';
            default:
                return 'bg-bg-card border-border-main';
        }
    }

    async function fetchPools() {
        if (!auth.isAuthenticated) return;
        isLoading = true;
        error = null;
        try {
            const res = await fetch('/api/v1/storage/pools');
            if (res.ok) {
                pools = await res.json();
            } else {
                const errText = await res.text();
                error = `Failed to load pools: ${res.status} ${errText}`;
            }
        } catch (e) {
            error = `Failed to load pools: ${e.message}`;
        } finally {
            isLoading = false;
        }
    }

    $effect(() => {
        if (auth.isAuthenticated) {
            fetchPools();
        }
    });
</script>

<div class="max-w-6xl mx-auto space-y-6">
    <!-- Header -->
    <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
            <h1 class="text-2xl font-bold text-text-main">{i18n.t('storage.title')}</h1>
            <p class="text-sm text-text-muted mt-1">{i18n.t('storage.subtitle')}</p>
        </div>
        <button
            onclick={fetchPools}
            disabled={isLoading}
            class="px-4 py-2 bg-primary text-primary-fg rounded hover:bg-primary/90 disabled:opacity-50 text-sm font-medium"
        >
            {i18n.t('storage.refresh')}
        </button>
    </div>

    <!-- Error state -->
    {#if error}
        <div class="bg-red-500/10 border border-red-500/30 rounded p-4 text-red-400">
            {error}
        </div>
    {/if}

    <!-- Loading state -->
    {#if isLoading && pools.length === 0}
        <div class="text-text-muted">{i18n.t('storage.loading')}</div>
    {/if}

    <!-- Empty state -->
    {#if !isLoading && pools.length === 0 && !error}
        <div class="bg-bg-card border border-border-main rounded p-8 text-center text-text-muted">
            {i18n.t('storage.empty')}
        </div>
    {/if}

    <!-- Pools list -->
    {#if pools.length > 0}
        <div class="space-y-4">
            {#each pools as pool (pool.name)}
                <div class="bg-bg-card border border-border-main rounded-lg p-4 shadow">
                    <div class="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4">
                        <!-- Pool name and health -->
                        <div class="flex items-center gap-4">
                            <div class="flex items-center gap-2">
                                <span class="text-lg font-bold text-text-main">{pool.name}</span>
                                <span class="px-2 py-0.5 text-xs font-medium rounded border {getHealthBg(pool.health)} {getHealthColor(pool.health)}">
                                    {pool.health}
                                </span>
                            </div>
                        </div>

                        <!-- Stats -->
                        <div class="flex flex-wrap gap-6 text-sm">
                            <div class="flex flex-col">
                                <span class="text-text-muted text-xs uppercase">{i18n.t('storage.capacity')}</span>
                                <span class="text-text-main font-medium">{pool.capacity}%</span>
                            </div>
                            <div class="flex flex-col">
                                <span class="text-text-muted text-xs uppercase">{i18n.t('storage.used')}</span>
                                <span class="text-text-main font-medium">{formatBytes(pool.allocated)}</span>
                            </div>
                            <div class="flex flex-col">
                                <span class="text-text-muted text-xs uppercase">{i18n.t('storage.free')}</span>
                                <span class="text-text-main font-medium">{formatBytes(pool.free)}</span>
                            </div>
                            <div class="flex flex-col">
                                <span class="text-text-muted text-xs uppercase">{i18n.t('storage.fragmentation')}</span>
                                <span class="text-text-main font-medium">{pool.fragmentation}%</span>
                            </div>
                        </div>
                    </div>

                    <!-- Capacity bar -->
                    <div class="mt-4">
                        <div class="h-2 bg-bg-main rounded-full overflow-hidden">
                            <div
                                class="h-full transition-all duration-300"
                                class:bg-green-500={pool.capacity < 70}
                                class:bg-yellow-500={pool.capacity >= 70 && pool.capacity < 85}
                                class:bg-red-500={pool.capacity >= 85}
                                style="width: {Math.min(100, pool.capacity)}%"
                            ></div>
                        </div>
                        <div class="flex justify-between mt-1 text-xs text-text-muted">
                            <span>{formatBytes(pool.allocated)} / {formatBytes(pool.size)}</span>
                            <span>{formatBytes(pool.free)} free</span>
                        </div>
                    </div>
                </div>
            {/each}
        </div>
    {/if}
</div>

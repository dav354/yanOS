<script>
    import { i18n } from '$lib/i18n.svelte.js';

    // Placeholder for future task feed (scrubs, replication, scheduled jobs)
    let tasks = $state([]);
</script>

<div class="bg-bg-sidebar text-text-sidebar border-t border-border-main flex flex-col transition-all duration-300 h-36">
    <div class="flex items-center justify-between px-2 py-1 bg-white/5 border-b border-border-main">
        <div>
            <div class="text-xs font-bold uppercase tracking-wide text-text-sidebar-muted">{i18n.t('tasks.title')}</div>
            <div class="text-[11px] text-text-sidebar-muted/80">{i18n.t('tasks.subtitle')}</div>
        </div>
    </div>
    <div class="flex-1 overflow-auto text-xs"></div>
</div>

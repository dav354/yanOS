<script>
    import { i18n } from '$lib/i18n.svelte.js';

    let tasks = $state([
        { name: 'ZFS scrub (rpool)', status: 'running', detail: 'Started 10:05, 12% done', severity: 'info' },
        { name: 'Replication to backup', status: 'scheduled', detail: 'Next run at 02:00', severity: 'warn' },
        { name: 'Snapshot rotation', status: 'idle', detail: 'Last run 04:12', severity: 'info' },
    ]);

    const badge = {
        running: 'bg-green-200 text-green-900',
        scheduled: 'bg-amber-200 text-amber-900',
        idle: 'bg-slate-200 text-slate-900',
    };
</script>

<div class="bg-bg-sidebar text-text-sidebar border-t border-border-main flex flex-col transition-all duration-300 h-36">
    <div class="flex items-center justify-between px-2 py-1 bg-white/5 border-b border-border-main">
        <div>
            <div class="text-xs font-bold uppercase tracking-wide text-text-sidebar-muted">{i18n.t('tasks.title')}</div>
            <div class="text-[11px] text-text-sidebar-muted/80">{i18n.t('tasks.subtitle')}</div>
        </div>
    </div>
    <div class="flex-1 overflow-auto text-xs">
        {#if tasks.length === 0}
            <div class="p-2 text-text-sidebar-muted italic">{i18n.t('tasks.none')}</div>
        {:else}
            <ul class="divide-y divide-border-main/60">
                {#each tasks as task}
                    <li class="px-3 py-2 hover:bg-white/5 transition-colors">
                        <div class="flex items-center justify-between">
                            <span class="font-semibold text-text-sidebar">{task.name}</span>
                            <span class={"px-2 py-0.5 rounded text-[11px] " + (badge[task.status] ?? 'bg-slate-200 text-slate-900')}>
                                {task.status}
                            </span>
                        </div>
                        <div class="text-text-sidebar-muted text-[11px] mt-0.5">{task.detail}</div>
                    </li>
                {/each}
            </ul>
        {/if}
    </div>
</div>

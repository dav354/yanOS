<script>
    import { auth } from '$lib/auth.svelte.js';

    let events = $state([]);
    let eventSocket = null;
    let isExpanded = $state(true);

    function connectEvents() {
        if (!auth.isAuthenticated) {
            if (eventSocket) {
                eventSocket.close();
                eventSocket = null;
            }
            return;
        }
        
        // Avoid double connection
        if (eventSocket && eventSocket.readyState <= 1) return;

        const protocol = location.protocol === 'https:' ? 'wss' : 'ws';
        const ws = new WebSocket(`${protocol}://${location.host}/api/v1/events`);
        eventSocket = ws;
        
        ws.onmessage = (evt) => {
            try {
                const payload = JSON.parse(evt.data);
                console.log('Log Event:', payload);
                // Ensure ts exists, default to now if missing
                if (!payload.ts) { 
                    payload.ts = new Date().toISOString(); 
                }
                events = [payload, ...events].slice(0, 200);
            } catch (e) {
                console.error('Failed to parse event', e);
            }
        };
        
        ws.onclose = () => {
            eventSocket = null;
        };
    }

    $effect(() => {
        if (auth.isAuthenticated) {
            connectEvents();
        }
        return () => {
            if (eventSocket) {
                eventSocket.close();
                eventSocket = null;
            }
        };
    });

    function toggle() {
        isExpanded = !isExpanded;
    }

    function formatTime(ts) {
        if (!ts) return '';
        try {
            return ts.split('T')[1].replace('Z','');
        } catch {
            return ts;
        }
    }
</script>

<div class="bg-bg-sidebar text-text-sidebar border-t border-border-main flex flex-col transition-all duration-300 {isExpanded ? 'h-48' : 'h-8'}">
    <!-- Header / Toggle Bar -->
    <button
        class="flex items-center justify-between px-2 py-1 bg-white/5 border-b border-border-main cursor-pointer select-none text-left"
        onclick={toggle}
        onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && toggle()}
        aria-expanded={isExpanded}
        type="button"
    >
        <span class="text-xs font-bold uppercase tracking-wide text-text-sidebar-muted">System Logs / Events</span>
        <span class="text-text-sidebar-muted hover:text-text-sidebar text-xs">
            {isExpanded ? '▼' : '▲'}
        </span>
    </button>

    <!-- Log Content -->
    <div class="flex-1 overflow-auto font-mono text-xs p-2 bg-black/20">
        {#if events.length === 0}
            <div class="text-text-sidebar-muted italic">No events received...</div>
        {:else}
            <table class="w-full text-left">
                <tbody>
                    {#each events as item}
                        <tr class="hover:bg-white/5">
                            <td class="whitespace-nowrap text-text-sidebar-muted py-0.5 w-40">{formatTime(item.ts)}</td>
                            <td class="whitespace-nowrap text-primary py-0.5 w-32 font-bold">{item.type || 'INFO'}</td>
                            <td class="text-text-sidebar py-0.5 break-all">
                                {JSON.stringify(item)}
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        {/if}
    </div>
</div>

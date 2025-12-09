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
                // Prepend new events (payload already has ts)
                events = [payload, ...events].slice(0, 200);
            } catch (e) {
                console.error('Failed to parse event', e);
            }
        };
        
        ws.onclose = () => {
            eventSocket = null;
            // Optional: Reconnect logic could go here
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
                    {#each events as event}
                        <tr class="hover:bg-white/5">
                            <td class="whitespace-nowrap text-text-sidebar-muted py-0.5 w-40">{event.ts.split('T')[1].replace('Z','')}</td>
                            <td class="whitespace-nowrap text-primary py-0.5 w-32 font-bold">{event.event?.type}</td>
                            <td class="text-text-sidebar py-0.5 break-all">
                                {#if event.event?.line}
                                    {event.event.line}
                                {:else if event.event?.path}
                                    <span class="text-yellow-600 mr-2">[{event.event.path}]</span>
                                {:else}
                                    {JSON.stringify(event.event ?? event).substring(0, 100)}...
                                {/if}
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        {/if}
    </div>
</div>

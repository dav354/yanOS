<script>
    import { onMount, onDestroy } from 'svelte';
    import { browser } from '$app/environment';
    import { i18n } from '$lib/i18n.svelte.js';
    import { auth } from '$lib/auth.svelte.js';

    let tasks = $state([]);
    let isExpanded = $state(true);
    let socket = null;
    let reconnectTimer = null;
    
    // Resizing state
    let height = $state(150);
    let isDragging = $state(false);
    let startY = 0;
    let startHeight = 0;

    function toggle() {
        if (isDragging) return; // Prevent toggle when ending a drag
        isExpanded = !isExpanded;
    }

    function onMouseDown(e) {
        // Only trigger on the top border/header area
        isDragging = true;
        startY = e.clientY;
        startHeight = height;
        document.body.style.userSelect = 'none'; // Prevent text selection
        e.preventDefault();
    }

    function onMouseMove(e) {
        if (!isDragging) return;
        const delta = startY - e.clientY; // Dragging up increases height
        const newHeight = startHeight + delta;
        // Clamp height
        if (newHeight >= 100 && newHeight <= 600) {
            height = newHeight;
        }
    }

    function onMouseUp() {
        if (isDragging) {
            isDragging = false;
            document.body.style.userSelect = '';
            if (browser) {
                localStorage.setItem('yanos-tasks-height', height.toString());
            }
        }
    }

    function formatTime(iso) {
        try {
            return new Date(iso).toLocaleTimeString();
        } catch {
            return iso;
        }
    }

    function formatDuration(ms) {
        if (ms < 1000) return `${ms}ms`;
        return `${(ms / 1000).toFixed(2)}s`;
    }

    function handleEvent(payload) {
        const type = payload.event?.type;
        if (!type) return;

        if (type === 'task_started') {
            const { id, name, started_at } = payload.event;
            tasks = [{
                id,
                name,
                started_at,
                status: 'running',
                progress: null
            }, ...tasks].slice(0, 50);
        } else if (type === 'task_completed') {
            const { id, duration_ms, status } = payload.event;
            // Update existing task
            const idx = tasks.findIndex(t => t.id === id);
            if (idx !== -1) {
                // Svelte 5 rune array mutation needs care if deep? 
                // Creating new array is safest for reactivity trigger
                const updated = { ...tasks[idx], status, duration_ms };
                const newTasks = [...tasks];
                newTasks[idx] = updated;
                tasks = newTasks;
            }
        }
    }

    function connect() {
        if (!auth.isAuthenticated) return;
        
        const protocol = location.protocol === 'https:' ? 'wss' : 'ws';
        socket = new WebSocket(`${protocol}://${location.host}/api/v1/events`);

        socket.onmessage = (evt) => {
            try {
                handleEvent(JSON.parse(evt.data));
            } catch (e) {
                console.error('TaskPanel: parse error', e);
            }
        };

        socket.onclose = () => {
            socket = null;
            if (auth.isAuthenticated) {
                reconnectTimer = setTimeout(connect, 3000);
            }
        };
    }

    onMount(() => {
        if (browser) {
            const saved = localStorage.getItem('yanos-tasks-height');
            if (saved) {
                const h = parseInt(saved);
                if (!isNaN(h) && h >= 100 && h <= 600) {
                    height = h;
                }
            }
        }
    });

    $effect(() => {
        if (auth.isAuthenticated && !socket) {
            connect();
        }
        return () => {
            if (socket) socket.close();
            if (reconnectTimer) clearTimeout(reconnectTimer);
        };
    });
</script>

<svelte:window onmousemove={onMouseMove} onmouseup={onMouseUp} />

<div 
    class="bg-bg-sidebar text-text-sidebar border-t border-border-main flex flex-col transition-all duration-75 relative"
    style="height: {isExpanded ? height + 'px' : '32px'}"
>
    <!-- Drag Handle -->
    {#if isExpanded}
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div 
            class="absolute top-0 left-0 right-0 h-1 cursor-ns-resize hover:bg-primary/50 z-10"
            onmousedown={onMouseDown}
            role="separator"
            aria-valuenow={height}
        ></div>
    {/if}

    <button
        class="flex items-center justify-between px-2 py-1 bg-white/5 border-b border-border-main cursor-pointer select-none text-left h-8 shrink-0"
        onclick={toggle}
        onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && toggle()}
        aria-expanded={isExpanded}
        type="button"
    >
        <span class="text-xs font-bold uppercase tracking-wide text-text-sidebar-muted">{i18n.t('tasks.title')}</span>
        <span class="text-text-sidebar-muted hover:text-text-sidebar text-xs">
            {isExpanded ? '▼' : '▲'}
        </span>
    </button>
    
    {#if isExpanded}
        <div class="flex-1 overflow-auto text-xs p-2 space-y-1">
            {#if tasks.length === 0}
                <div class="text-text-sidebar-muted italic text-center py-2">{i18n.t('tasks.none')}</div>
            {:else}
                {#each tasks as task (task.id)}
                    <div class="flex items-center justify-between py-1 border-b border-white/5 last:border-0">
                        <div class="flex flex-col">
                            <span class="font-medium text-text-sidebar">{task.name}</span>
                            <span class="text-[10px] text-text-sidebar-muted">{formatTime(task.started_at)}</span>
                        </div>
                        <div class="flex items-center gap-2">
                            {#if task.status === 'running'}
                                <span class="text-primary animate-pulse">Running...</span>
                            {:else if task.status === 'success'}
                                <span class="text-green-400">Done ({formatDuration(task.duration_ms)})</span>
                            {:else}
                                <span class="text-red-400">Failed ({formatDuration(task.duration_ms)})</span>
                            {/if}
                        </div>
                    </div>
                {/each}
            {/if}
        </div>
    {/if}
</div>

<script>
    import { onMount, onDestroy } from 'svelte';
    import '@xterm/xterm/css/xterm.css';
    import { auth } from '$lib/auth.svelte.js';
    import { browser } from '$app/environment';
    import { theme } from '$lib/theme.svelte.js';

    let terminalContainer = $state(null);
    let term = $state(null);
    let socket = $state(null);
    let fitAddon = $state(null);
    let started = $state(false);

    $effect(() => {
        if (browser && term && theme.current) {
            // Wait for the DOM to update with the new data-theme attribute
            requestAnimationFrame(() => {
                const computed = getComputedStyle(document.documentElement);
                const background = computed.getPropertyValue('--bg-sidebar').trim();
                const foreground = computed.getPropertyValue('--text-sidebar').trim();
                
                if (background && foreground) {
                    term.options.theme = {
                        background,
                        foreground
                    };
                }
            });
        }
    });

    function resizeTerminal() {
        if (fitAddon && term) {
            fitAddon.fit();
            if (socket && socket.readyState === WebSocket.OPEN) {
                socket.send(JSON.stringify({
                    type: 'resize',
                    rows: term.rows,
                    cols: term.cols
                }));
            }
        }
    }

    async function startTerminal() {
        if (started || !browser || !auth.isAuthenticated || !terminalContainer) return;
        started = true;

        const [{ Terminal }, { FitAddon }, { WebLinksAddon }] = await Promise.all([
            import('@xterm/xterm'),
            import('@xterm/addon-fit'),
            import('@xterm/addon-web-links'),
        ]);

        term = new Terminal({
            cursorBlink: true,
            theme: {
                // Initial fallback, will be updated by the $effect immediately
                background: '#1f2937',
                foreground: '#ffffff',
            },
            fontFamily: 'Menlo, Monaco, "Courier New", monospace',
            fontSize: 14
        });

        fitAddon = new FitAddon();
        term.loadAddon(fitAddon);
        term.loadAddon(new WebLinksAddon());

        term.open(terminalContainer);
        fitAddon.fit();

        // Connect WebSocket
        const protocol = location.protocol === 'https:' ? 'wss' : 'ws';
        socket = new WebSocket(`${protocol}://${location.host}/api/v1/terminal`);
        socket.binaryType = 'arraybuffer';

        socket.onopen = () => {
            term.write('\r\n\x1b[32mConnected to yanOS Terminal\x1b[0m\r\n');
            resizeTerminal();

            // Auto-run command from URL if present
            const params = new URLSearchParams(window.location.search);
            const cmd = params.get('cmd');
            if (cmd) {
                // Small delay to ensure shell is ready for input
                setTimeout(() => {
                    socket.send(JSON.stringify({
                        type: 'input',
                        data: cmd + '\r'
                    }));
                    // Clear the query param so refresh doesn't re-run it
                    window.history.replaceState({}, '', location.pathname);
                }, 500);
            }
        };

        socket.onmessage = (event) => {
            if (event.data instanceof ArrayBuffer) {
                term.write(new Uint8Array(event.data));
            } else {
                term.write(event.data);
            }
        };

        socket.onclose = () => {
            term.write('\r\n\x1b[31mConnection closed\x1b[0m\r\n');
        };

        term.onData(data => {
            if (socket && socket.readyState === WebSocket.OPEN) {
                // Send as simple input message
                socket.send(JSON.stringify({
                    type: 'input',
                    data: data
                }));
            }
        });

        // Handle resize
        window.addEventListener('resize', resizeTerminal);
    }

    onMount(() => {
        // Defer start until auth is ready and container exists
        startTerminal();
    });

    $effect(() => {
        if (auth.isInitialized && auth.isAuthenticated) {
            startTerminal();
        }
    });

    onDestroy(() => {
        if (socket) socket.close();
        if (term) term.dispose();
        if (browser) {
            window.removeEventListener('resize', resizeTerminal);
        }
    });
</script>

<div class="h-full flex flex-col p-4 space-y-4">
    <h1 class="text-3xl font-bold text-text-main">Web Terminal</h1>
    
    <div class="flex-1 bg-bg-sidebar rounded-lg overflow-hidden shadow-inner border border-border-main relative">
        <div bind:this={terminalContainer} class="absolute inset-0 p-2"></div>
    </div>
</div>

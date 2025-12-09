<script>
    import { onMount, onDestroy } from 'svelte';
    import '@xterm/xterm/css/xterm.css';
    import { auth } from '$lib/auth.svelte.js';
    import { browser } from '$app/environment';

    let terminalContainer = $state(null);
    let term = $state(null);
    let socket = $state(null);
    let fitAddon = $state(null);

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

    onMount(async () => {
        if (!browser || !auth.isAuthenticated) {
            return;
        }

        const [{ Terminal }, { FitAddon }, { WebLinksAddon }] = await Promise.all([
            import('@xterm/xterm'),
            import('@xterm/addon-fit'),
            import('@xterm/addon-web-links'),
        ]);

        term = new Terminal({
            cursorBlink: true,
            theme: {
                background: '#1f2937', // Match bg-sidebar
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
            term.write('\r\n\x1b[32mConnected to zOS Terminal\x1b[0m\r\n');
            resizeTerminal();
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

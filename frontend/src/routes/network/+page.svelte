<script>
    /**
     * Network Configuration Page
     *
     * Displays network interfaces with IP, MAC, speed info.
     * Allows editing per-interface settings (static IP or DHCP).
     * Shows system-wide config: DNS servers, search domains, gateway.
     */
    import { auth } from '$lib/auth.svelte.js';
    import { i18n } from '$lib/i18n.svelte.js';

    let interfaces = $state([]);
    let config = $state({ dns_servers: [], dns_search: [], gateway: '', hostname: '' });
    let isLoading = $state(false);
    let isSaving = $state(false);
    let message = $state(null);
    let messageType = $state('success');

    // Edit modal state
    let editingInterface = $state(null);
    let editForm = $state({ address: '', prefix_len: 24, useDhcp: false, mtu: 1500 });

    // System config form
    let dnsInput = $state('');
    let gatewayInput = $state('');
    let hostnameInput = $state('');

    async function fetchData() {
        if (!auth.isAuthenticated) return;
        isLoading = true;
        message = null;
        try {
            const [ifaceRes, configRes] = await Promise.all([
                fetch('/api/v1/network/interfaces'),
                fetch('/api/v1/network/config')
            ]);

            if (ifaceRes.ok) {
                interfaces = await ifaceRes.json();
            }
            if (configRes.ok) {
                config = await configRes.json();
                dnsInput = config.dns_servers.join(', ');
                gatewayInput = config.gateway || '';
                hostnameInput = config.hostname || '';
            }
        } catch (e) {
            console.error('Failed to load network data', e);
        } finally {
            isLoading = false;
        }
    }

    function formatSpeed(speed) {
        if (!speed || speed === 0) return '-';
        if (speed >= 1000) return `${speed / 1000} Gbps`;
        return `${speed} Mbps`;
    }

    function openEditModal(iface) {
        editingInterface = iface;
        editForm = {
            address: iface.address || '',
            prefix_len: iface.prefix_len || 24,
            useDhcp: iface.addr_type === 'dhcp',
            mtu: iface.mtu || 1500
        };
    }

    function isInterfaceUp(iface) {
        // Consider interface up if it has an address and the state is up/ok
        const state = (iface.state || '').toLowerCase();
        return state === 'up' || state === 'ok' || (iface.address && iface.address.length > 0);
    }

    function closeEditModal() {
        editingInterface = null;
    }

    async function saveInterfaceConfig() {
        if (!editingInterface) return;
        isSaving = true;
        message = null;

        try {
            // Save IP configuration
            const endpoint = editForm.useDhcp
                ? `/api/v1/network/interface/${editingInterface.name}/dhcp`
                : `/api/v1/network/interface/${editingInterface.name}/address`;

            const options = {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
            };

            if (!editForm.useDhcp) {
                options.body = JSON.stringify({
                    address: editForm.address,
                    prefix_len: parseInt(editForm.prefix_len, 10)
                });
            }

            const res = await fetch(endpoint, options);
            if (!res.ok) {
                const err = await res.json();
                message = err.message || i18n.t('network.applyError');
                messageType = 'error';
                isSaving = false;
                return;
            }

            // Save MTU if changed
            const currentMtu = editingInterface.mtu || 1500;
            if (parseInt(editForm.mtu, 10) !== currentMtu) {
                const mtuRes = await fetch(`/api/v1/network/interface/${editingInterface.name}/mtu`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ mtu: parseInt(editForm.mtu, 10) })
                });
                if (!mtuRes.ok) {
                    const err = await mtuRes.json();
                    message = err.message || 'Failed to set MTU';
                    messageType = 'error';
                    isSaving = false;
                    return;
                }
            }

            message = i18n.t('network.applySuccess');
            messageType = 'success';
            closeEditModal();
            await fetchData();
        } catch (e) {
            message = i18n.t('network.applyError');
            messageType = 'error';
        } finally {
            isSaving = false;
        }
    }

    async function applySystemConfig() {
        isSaving = true;
        message = null;

        try {
            // Save hostname if changed
            if (hostnameInput.trim() && hostnameInput.trim() !== config.hostname) {
                const hostnameRes = await fetch('/api/v1/network/hostname', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ hostname: hostnameInput.trim() })
                });
                if (!hostnameRes.ok) {
                    const err = await hostnameRes.json();
                    message = err.message || 'Failed to set hostname';
                    messageType = 'error';
                    isSaving = false;
                    return;
                }
            }

            // Save DNS and gateway
            const dns_servers = dnsInput
                .split(/[,\s]+/)
                .map(s => s.trim())
                .filter(s => s.length > 0);

            const body = {
                dns_servers,
                gateway: gatewayInput.trim() || null
            };

            const res = await fetch('/api/v1/network/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(body)
            });

            if (res.ok) {
                message = i18n.t('network.applySuccess');
                messageType = 'success';
                await fetchData();
            } else {
                const err = await res.json();
                message = err.message || i18n.t('network.applyError');
                messageType = 'error';
            }
        } catch (e) {
            message = i18n.t('network.applyError');
            messageType = 'error';
        } finally {
            isSaving = false;
        }
    }

    $effect(() => {
        if (auth.isAuthenticated) {
            fetchData();
        }
    });
</script>

<div class="space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
        <div>
            <h1 class="text-2xl font-bold text-text-main">{i18n.t('network.title')}</h1>
            <p class="text-sm text-text-muted mt-1">{i18n.t('network.subtitle')}</p>
        </div>
        <button
            onclick={fetchData}
            class="text-primary hover:text-primary-hover text-sm font-medium flex items-center gap-1"
        >
            <span class="text-lg">&#8635;</span> {i18n.t('network.refresh')}
        </button>
    </div>

    <!-- Message -->
    {#if message}
        <div class="p-3 rounded-lg {messageType === 'success' ? 'bg-green-100 text-green-800 border border-green-200' : 'bg-red-100 text-red-800 border border-red-200'}">
            {message}
        </div>
    {/if}

    <!-- Interfaces Card -->
    <div class="bg-bg-card shadow-md rounded-lg overflow-hidden border border-border-main">
        <div class="px-6 py-4 border-b border-border-main bg-bg-main">
            <h2 class="text-lg font-semibold text-text-main">{i18n.t('network.interfaces')}</h2>
        </div>

        {#if isLoading && interfaces.length === 0}
            <div class="p-8 text-center text-text-muted">{i18n.t('network.loading')}</div>
        {:else if interfaces.length === 0}
            <div class="p-8 text-center text-text-muted">{i18n.t('network.empty')}</div>
        {:else}
            <div class="overflow-x-auto">
                <table class="min-w-full divide-y divide-border-main">
                    <thead class="bg-bg-main">
                        <tr>
                            <th class="px-4 py-3 text-left text-xs font-medium text-text-muted uppercase">{i18n.t('network.interface')}</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-text-muted uppercase">{i18n.t('network.state')}</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-text-muted uppercase">{i18n.t('network.ipAddress')}</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-text-muted uppercase">{i18n.t('network.type')}</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-text-muted uppercase">{i18n.t('network.mac')}</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-text-muted uppercase">{i18n.t('network.speed')}</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-text-muted uppercase">{i18n.t('network.mtu')}</th>
                            <th class="px-4 py-3 text-right text-xs font-medium text-text-muted uppercase"></th>
                        </tr>
                    </thead>
                    <tbody class="bg-bg-card divide-y divide-border-main">
                        {#each interfaces as iface (iface.name)}
                            <tr class="hover:bg-bg-main transition-colors">
                                <td class="px-4 py-3 whitespace-nowrap">
                                    <span class="font-mono font-medium text-text-main">{iface.name}</span>
                                    {#if iface.friendly_name}
                                        <span class="text-xs text-text-muted ml-2">({iface.friendly_name})</span>
                                    {/if}
                                </td>
                                <td class="px-4 py-3 whitespace-nowrap">
                                    <span class="px-2 py-0.5 inline-flex text-xs leading-5 font-semibold rounded-full
                                        {isInterfaceUp(iface) ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}">
                                        {isInterfaceUp(iface) ? i18n.t('network.up') : i18n.t('network.down')}
                                    </span>
                                </td>
                                <td class="px-4 py-3 whitespace-nowrap font-mono text-sm text-text-main">
                                    {#if iface.address}
                                        {iface.address}{#if iface.prefix_len}/{iface.prefix_len}{/if}
                                    {:else}
                                        <span class="text-text-muted">{i18n.t('network.noAddress')}</span>
                                    {/if}
                                </td>
                                <td class="px-4 py-3 whitespace-nowrap text-sm">
                                    {#if iface.addr_type === 'dhcp'}
                                        <span class="text-blue-600">{i18n.t('network.dhcp')}</span>
                                    {:else if iface.addr_type === 'static'}
                                        <span class="text-purple-600">{i18n.t('network.static')}</span>
                                    {:else}
                                        <span class="text-text-muted">-</span>
                                    {/if}
                                </td>
                                <td class="px-4 py-3 whitespace-nowrap font-mono text-xs text-text-muted">
                                    {iface.mac || '-'}
                                </td>
                                <td class="px-4 py-3 whitespace-nowrap text-sm text-text-muted">
                                    {formatSpeed(iface.speed)}
                                </td>
                                <td class="px-4 py-3 whitespace-nowrap text-sm text-text-muted">
                                    {iface.mtu || '-'}
                                </td>
                                <td class="px-4 py-3 whitespace-nowrap text-right">
                                    <button
                                        onclick={() => openEditModal(iface)}
                                        class="text-primary hover:text-primary-hover text-sm font-medium"
                                    >
                                        {i18n.t('network.editInterface')}
                                    </button>
                                </td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        {/if}
    </div>

    <!-- System Network Config Card -->
    <div class="bg-bg-card shadow-md rounded-lg overflow-hidden border border-border-main">
        <div class="px-6 py-4 border-b border-border-main bg-bg-main">
            <h2 class="text-lg font-semibold text-text-main">{i18n.t('network.systemConfig')}</h2>
        </div>

        <div class="p-6 space-y-4">
            <!-- Hostname -->
            <div>
                <label for="hostname-input" class="block text-sm font-medium text-text-muted mb-1">{i18n.t('network.hostname')}</label>
                <input
                    id="hostname-input"
                    type="text"
                    bind:value={hostnameInput}
                    placeholder="myhost"
                    class="w-full px-3 py-2 bg-bg-card border border-border-main rounded-lg text-text-main focus:ring-2 focus:ring-primary focus:border-transparent"
                />
                <p class="text-xs text-text-muted mt-1">Alphanumeric, hyphens, and dots only</p>
            </div>

            <!-- DNS Servers -->
            <div>
                <label for="dns-input" class="block text-sm font-medium text-text-muted mb-1">{i18n.t('network.dns')}</label>
                <input
                    id="dns-input"
                    type="text"
                    bind:value={dnsInput}
                    placeholder={i18n.t('network.dnsPlaceholder')}
                    class="w-full px-3 py-2 bg-bg-card border border-border-main rounded-lg text-text-main focus:ring-2 focus:ring-primary focus:border-transparent"
                />
                <p class="text-xs text-text-muted mt-1">Comma or space separated</p>
            </div>

            <!-- Search Domains (read-only for now) -->
            {#if config.dns_search?.length > 0}
                <div>
                    <label for="dns-search-input" class="block text-sm font-medium text-text-muted mb-1">{i18n.t('network.dnsSearch')}</label>
                    <input
                        id="dns-search-input"
                        type="text"
                        value={config.dns_search.join(', ')}
                        disabled
                        class="w-full px-3 py-2 bg-bg-main border border-border-main rounded-lg text-text-main opacity-60"
                    />
                </div>
            {/if}

            <!-- Gateway -->
            <div>
                <label for="gateway-input" class="block text-sm font-medium text-text-muted mb-1">{i18n.t('network.gateway')}</label>
                <input
                    id="gateway-input"
                    type="text"
                    bind:value={gatewayInput}
                    placeholder={i18n.t('network.gatewayPlaceholder')}
                    class="w-full px-3 py-2 bg-bg-card border border-border-main rounded-lg text-text-main focus:ring-2 focus:ring-primary focus:border-transparent"
                />
            </div>

            <!-- Apply Button -->
            <div class="pt-4">
                <button
                    onclick={applySystemConfig}
                    disabled={isSaving}
                    class="px-4 py-2 bg-primary text-white rounded-lg hover:bg-primary-hover disabled:opacity-50 disabled:cursor-not-allowed font-medium"
                >
                    {isSaving ? i18n.t('network.applying') : i18n.t('network.apply')}
                </button>
            </div>
        </div>
    </div>
</div>

<!-- Edit Interface Modal -->
{#if editingInterface}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <div
        class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
        role="dialog"
        aria-modal="true"
        aria-labelledby="edit-modal-title"
        onclick={closeEditModal}
        onkeydown={(e) => e.key === 'Escape' && closeEditModal()}
    >
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
            class="bg-bg-card rounded-lg shadow-xl border border-border-main w-full max-w-md mx-4"
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.stopPropagation()}
        >
            <div class="px-6 py-4 border-b border-border-main">
                <h3 id="edit-modal-title" class="text-lg font-semibold text-text-main">
                    {i18n.t('network.editInterface')}: {editingInterface.name}
                </h3>
            </div>

            <div class="p-6 space-y-4">
                <!-- DHCP Toggle -->
                <div class="flex items-center gap-3">
                    <label class="flex items-center gap-2 cursor-pointer">
                        <input
                            type="radio"
                            name="addrType"
                            checked={editForm.useDhcp}
                            onchange={() => editForm.useDhcp = true}
                            class="text-primary"
                        />
                        <span class="text-text-main">{i18n.t('network.useDhcp')}</span>
                    </label>
                    <label class="flex items-center gap-2 cursor-pointer">
                        <input
                            type="radio"
                            name="addrType"
                            checked={!editForm.useDhcp}
                            onchange={() => editForm.useDhcp = false}
                            class="text-primary"
                        />
                        <span class="text-text-main">{i18n.t('network.useStatic')}</span>
                    </label>
                </div>

                <!-- IP Address with Prefix -->
                <div class={editForm.useDhcp ? 'opacity-50' : ''}>
                    <label for="edit-ip-address" class="block text-sm font-medium text-text-muted mb-1">{i18n.t('network.ipAddress')}</label>
                    <div class="flex items-center gap-1">
                        <input
                            id="edit-ip-address"
                            type="text"
                            bind:value={editForm.address}
                            placeholder="192.168.1.10"
                            disabled={editForm.useDhcp}
                            class="flex-1 px-3 py-2 bg-bg-card border border-border-main rounded-lg text-text-main focus:ring-2 focus:ring-primary font-mono disabled:cursor-not-allowed"
                        />
                        <span class="text-text-muted text-lg">/</span>
                        <input
                            id="edit-prefix-len"
                            type="text"
                            bind:value={editForm.prefix_len}
                            disabled={editForm.useDhcp}
                            class="w-16 px-3 py-2 bg-bg-card border border-border-main rounded-lg text-text-main focus:ring-2 focus:ring-primary font-mono text-center disabled:cursor-not-allowed"
                        />
                    </div>
                    <p class="text-xs text-text-muted mt-1">{editForm.useDhcp ? 'Assigned by DHCP server' : 'e.g., 192.168.1.10 / 24'}</p>
                </div>

                <!-- MTU -->
                <div>
                    <label for="edit-mtu" class="block text-sm font-medium text-text-muted mb-1">MTU</label>
                    <input
                        id="edit-mtu"
                        type="text"
                        bind:value={editForm.mtu}
                        class="w-full px-3 py-2 bg-bg-card border border-border-main rounded-lg text-text-main focus:ring-2 focus:ring-primary"
                    />
                    <p class="text-xs text-text-muted mt-1">576-9000 (9000 for jumbo frames)</p>
                </div>
            </div>

            <div class="px-6 py-4 border-t border-border-main flex justify-end gap-3">
                <button
                    onclick={closeEditModal}
                    class="px-4 py-2 text-text-muted hover:text-text-main"
                >
                    {i18n.t('network.cancel')}
                </button>
                <button
                    onclick={saveInterfaceConfig}
                    disabled={isSaving}
                    class="px-4 py-2 bg-primary text-white rounded-lg hover:bg-primary-hover disabled:opacity-50"
                >
                    {isSaving ? i18n.t('network.applying') : i18n.t('network.save')}
                </button>
            </div>
        </div>
    </div>
{/if}

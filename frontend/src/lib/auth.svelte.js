/**
 * Authentication state management using Svelte 5 runes.
 *
 * This module provides a singleton AuthStore that:
 * - Initializes auth state from /api/v1/status on load
 * - Provides login/logout methods that communicate with the backend
 * - Tracks CSRF tokens for protected mutations
 * - Exposes reactive isAuthenticated derived state
 *
 * Usage:
 *   import { auth } from '$lib/auth.svelte.js';
 *   if (auth.isAuthenticated) { ... }
 *   await auth.login(username, password);
 *   await auth.logout();
 */

import { browser } from '$app/environment';

/**
 * Reactive authentication store using Svelte 5 class-based runes pattern.
 */
class AuthStore {
    /** Currently authenticated username, or null if not logged in */
    user = $state(null);
    /** CSRF token from cookie, used for mutation requests */
    csrfToken = $state(null);
    /** True once initial auth check has completed */
    isInitialized = $state(false);
    /** Derived: true if user is authenticated */
    isAuthenticated = $derived(!!this.user);

    constructor() {
        if (browser) {
            this.init();
        }
    }

    async init() {
        try {
            const controller = new AbortController();
            const timeout = setTimeout(() => controller.abort(), 4000);
            const res = await fetch('/api/v1/status', { credentials: 'include', signal: controller.signal });
            if (res.ok) {
                const data = await res.json();
                this.csrfToken = this.readCsrfFromCookie();
                this.user = data.user ?? null;
            }
            clearTimeout(timeout);
        } catch (e) {
            console.error('Failed to init auth', e);
        } finally {
            this.isInitialized = true;
        }
    }

    readCsrfFromCookie() {
        if (!browser) return null;
        const match = document.cookie.match(/(?:^|;\s*)XSRF-TOKEN=([^;]+)/);
        return match ? decodeURIComponent(match[1]) : null;
    }

    async login(username, password) {
        if (!this.csrfToken) {
            await this.init();
        }

        const token = this.csrfToken ?? this.readCsrfFromCookie();

        const res = await fetch('/api/v1/login', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                ...(token ? { 'X-CSRF-TOKEN': token } : {})
            },
            credentials: 'include',
            body: JSON.stringify({ username, password })
        });

        if (res.ok) {
            this.csrfToken = this.readCsrfFromCookie();
            await this.init();
            this.user = username;
            return true;
        } else {
            this.user = null;
            console.error('Login failed');
            return false;
        }
    }

    async logout() {
        try {
            const token = this.csrfToken ?? this.readCsrfFromCookie();
            await fetch('/api/v1/logout', {
                method: 'POST',
                headers: {
                    ...(token ? { 'X-CSRF-TOKEN': token } : {})
                },
                credentials: 'include'
            });
        } catch (e) {
            console.error('Logout request failed', e);
        } finally {
            this.user = null;
            this.csrfToken = null;
        }
    }
}

export const auth = new AuthStore();

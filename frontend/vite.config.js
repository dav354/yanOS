import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import path from 'node:path';

export default defineConfig({
	plugins: [sveltekit()],
	resolve: {
		alias: {
			'#client': path.resolve('node_modules/svelte/src/internal/client')
		}
	},
	server: {
		proxy: {
			'/api': {
				target: 'https://127.0.0.1:8443',
				changeOrigin: true,
				secure: false
			},
			'/healthz': {
				target: 'https://127.0.0.1:8443',
				changeOrigin: true,
				secure: false
			},
			'/readyz': {
				target: 'https://127.0.0.1:8443',
				changeOrigin: true,
				secure: false
			}
		}
	}
});

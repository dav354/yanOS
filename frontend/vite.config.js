import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
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

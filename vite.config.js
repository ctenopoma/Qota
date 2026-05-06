import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath, URL } from 'node:url';
// Tauri 開発時はポート 1423 / strict / HMR over IPC を推奨
var host = process.env.TAURI_DEV_HOST;
export default defineConfig({
    plugins: [react()],
    resolve: {
        alias: {
            '@': fileURLToPath(new URL('./src', import.meta.url))
        }
    },
    clearScreen: false,
    server: {
        port: 1423,
        strictPort: true,
        host: host || false,
        hmr: host
            ? { protocol: 'ws', host: host, port: 1421 }
            : undefined,
        watch: {
            // Rust 側はビルド対象外
            ignored: ['**/src-tauri/**'],
        },
    },
});

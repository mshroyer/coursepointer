import { defineConfig } from 'vite'

export default defineConfig({
    server: {
        host: '0.0.0.0', // Listen on all network interfaces
        port: 5173,       // Optional: default is 5173
    },
});

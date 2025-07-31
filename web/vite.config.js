import { defineConfig } from 'vite'
import basicSsl from '@vitejs/plugin-basic-ssl'

export default defineConfig({
    plugins: [
        basicSsl()
    ],
    server: {
        host: '0.0.0.0', // Listen on all network interfaces
        port: 5173,       // Optional: default is 5173
    },
});

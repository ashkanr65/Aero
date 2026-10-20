import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'node:path';
export default defineConfig({plugins:[react()],root:'src/renderer',base:'./',server:{port:5174,strictPort:true},build:{outDir:'../../dist',emptyOutDir:true,rollupOptions:{input:{main:path.resolve('src/renderer/index.html'),passwords:path.resolve('src/renderer/password-manager.html')}}}});

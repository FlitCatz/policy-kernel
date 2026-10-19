import { defineConfig } from 'vite';
export default defineConfig({server:{host:'127.0.0.1',port:4180,strictPort:true,proxy:{'/api':'http://127.0.0.1:5077'}}});

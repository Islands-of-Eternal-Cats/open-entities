import path from 'path';
import fs from 'fs';
import { fileURLToPath } from 'url';
import { spawn } from 'child_process';
import { defineConfig } from 'vite';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const rustDirs = [
    path.resolve(process.cwd(), '../open-entities-lib'),
    path.resolve(process.cwd(), '../wasm-bindings')
];

export default defineConfig({
    resolve: {
        alias: {
            'open-entities-wasm': path.resolve(__dirname, 'node_modules/open-entities-wasm/wasm_bindings.js'),
        },
    },
    optimizeDeps: {
        exclude: ['open-entities-wasm'],
    },
    server: {
        port: 5173,
        open: true
    },
    build: {
        outDir: 'dist',
        sourcemap: true
    },
    plugins: [
        {
            name: 'serve-assets-from-repo',
            configureServer(server) {
                const assetsDir = path.resolve(server.config.root, '../assets');
                server.middlewares.use('/assets', (req, res, next) => {
                    const raw = (req.url || '').replace(/^\//, '');
                    const subPath = raw.startsWith('assets/') ? raw.slice(7) : raw;
                    if (!subPath) return next();
                    const file = path.join(assetsDir, subPath);
                    if (!fs.existsSync(file) || !fs.statSync(file).isFile()) return next();
                    res.setHeader('Cache-Control', 'no-store');
                    fs.createReadStream(file).pipe(res);
                });
            }
        },
        {
            name: 'serve-wasm-with-mime',
            configureServer(server) {
                const publicDir = path.resolve(server.config.root, 'public');
                const wasmPath = path.join(publicDir, 'wasm_bindings_bg.wasm');
                const wasmMiddleware = (req, res, next) => {
                    if (req.url !== '/wasm_bindings_bg.wasm' && !req.url.startsWith('/wasm_bindings_bg.wasm?')) {
                        return next();
                    }
                    if (!fs.existsSync(wasmPath)) return next();
                    res.setHeader('Content-Type', 'application/wasm');
                    res.setHeader('Cache-Control', 'no-store');
                    fs.createReadStream(wasmPath).pipe(res);
                };
                server.middlewares.stack.unshift({ route: '', handle: wasmMiddleware });
            }
        },
        {
            name: 'watch-rust-dirs',
            configureServer(server) {
                let buildTimeout = null;
                const script = path.resolve(process.cwd(), 'build-wasm.sh');
                const runWasmBuild = () => {
                    const child = spawn('sh', [script], { cwd: process.cwd(), stdio: 'inherit' });
                    child.on('close', (code) => {
                        if (code === 0) {
                            console.log('[watch-rust] WASM rebuild done. Reload the page to use the new build.');
                            server.ws.send({ type: 'full-reload', path: '*' });
                        }
                    });
                };
                rustDirs.forEach(dir => server.watcher.add(dir));
                server.watcher.on('change', (file) => {
                    if (!file.endsWith('.rs')) return;
                    if (buildTimeout) clearTimeout(buildTimeout);
                    buildTimeout = setTimeout(() => {
                        buildTimeout = null;
                        console.log('[watch-rust] Rust file changed, rebuilding WASM...');
                        runWasmBuild();
                    }, 300);
                });
            }
        }
    ]
});

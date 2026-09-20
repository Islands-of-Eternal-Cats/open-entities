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
            'open_entities_wasm': path.resolve(__dirname, 'node_modules/open_entities_wasm/open_entities_wasm.js'),
        },
    },
    optimizeDeps: {
        exclude: ['open_entities_wasm'],
    },
    server: {
        port: 5173,
        // Fail instead of quietly moving to 5174: a second dev server on another port keeps
        // serving the browser an older build from the port the tab is already on, and the app
        // then complains about methods the WASM module "does not have".
        strictPort: true,
        open: true
    },
    build: {
        outDir: 'dist',
        sourcemap: true
    },
    plugins: [
        {
            name: 'serve-wasm-with-mime',
            configureServer(server) {
                const publicDir = path.resolve(server.config.root, 'public');
                const wasmPath = path.join(publicDir, 'open_entities_wasm_bg.wasm');
                const wasmMiddleware = (req, res, next) => {
                    if (req.url !== '/open_entities_wasm_bg.wasm' && !req.url.startsWith('/open_entities_wasm_bg.wasm?')) {
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
                // Not just 'change': git writes a temp file and renames it over the target, so
                // applying a patch or switching branches arrives as unlink + add. Listening for
                // 'change' alone meant `git am` never rebuilt the wasm, and the browser kept
                // running the previous core while the source on disk said otherwise.
                const onRustFile = (file) => {
                    if (!file.endsWith('.rs')) return;
                    if (buildTimeout) clearTimeout(buildTimeout);
                    buildTimeout = setTimeout(() => {
                        buildTimeout = null;
                        console.log('[watch-rust] Rust sources changed, rebuilding WASM...');
                        runWasmBuild();
                    }, 300);
                };
                ['change', 'add', 'unlink'].forEach(event => server.watcher.on(event, onRustFile));
            }
        }
    ]
});

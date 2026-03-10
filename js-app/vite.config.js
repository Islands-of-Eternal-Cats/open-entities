import path from 'path';
import { spawn } from 'child_process';
import { defineConfig } from 'vite';
import wasmPackWatchPlugin from 'vite-plugin-wasm-pack-watcher';

const rustDirs = [
    path.resolve(process.cwd(), '../open-entities-lib'),
    path.resolve(process.cwd(), '../wasm-bindings')
];

function runWasmBuild() {
    const script = path.resolve(process.cwd(), 'build-wasm.sh');
    const child = spawn('sh', [script], { cwd: process.cwd(), stdio: 'inherit' });
    child.on('close', (code) => {
        if (code === 0) {
            console.log('[watch-rust] WASM rebuild done. Reload the page to use the new build.');
        }
    });
}

export default defineConfig({
    server: {
        port: 5173,
        open: true
    },
    build: {
        outDir: 'dist',
        sourcemap: true,
        watch: {
            include: ['src/**', '../open-entities-lib/**/*.rs', '../wasm-bindings/**/*.rs']
        }
    },
    plugins: [
        wasmPackWatchPlugin({
            buildCommand: './build-wasm.sh'
        }),
        {
            name: 'watch-rust-dirs',
            configureServer(server) {
                let buildTimeout = null;
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

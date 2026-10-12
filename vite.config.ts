import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import fs from 'node:fs';
import path from 'node:path';
export default defineConfig({
  plugins: [
    react(),
    {
      name: 'bundled-receipt-ocr',
      generateBundle() {
        const files = [
          ['node_modules/tesseract.js/dist/worker.min.js', 'worker.min.js'],
          ['node_modules/@tesseract.js-data/eng/4.0.0/eng.traineddata.gz', 'eng.traineddata.gz'],
        ];
        for (const name of fs
          .readdirSync('node_modules/tesseract.js-core')
          .filter((n) => n.endsWith('-lstm.wasm.js')))
          files.push([path.join('node_modules/tesseract.js-core', name), name]);
        for (const [source, name] of files)
          this.emitFile({
            type: 'asset',
            fileName: 'ocr/' + name,
            source: fs.readFileSync(source),
          });
      },
      configureServer(server) {
        server.middlewares.use('/ocr', (req, res, next) => {
          const name = (req.url || '').split('?')[0].slice(1);
          const source =
            name === 'worker.min.js'
              ? 'node_modules/tesseract.js/dist/worker.min.js'
              : name === 'eng.traineddata.gz'
                ? 'node_modules/@tesseract.js-data/eng/4.0.0/eng.traineddata.gz'
                : path.join('node_modules/tesseract.js-core', name);
          if (!/^[a-z0-9.\-]+$/i.test(name) || !fs.existsSync(source)) return next();
          res.setHeader(
            'Content-Type',
            name.endsWith('.js')
              ? 'application/javascript'
              : name.endsWith('.wasm')
                ? 'application/wasm'
                : 'application/octet-stream',
          );
          res.end(fs.readFileSync(source));
        });
      },
    },
  ],
  server: { port: 1420, strictPort: true, proxy: { '/api': 'http://127.0.0.1:3030' } },
  clearScreen: false,
});

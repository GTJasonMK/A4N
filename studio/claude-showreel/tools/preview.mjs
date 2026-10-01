import { serve } from './browser.mjs';
const server=await serve();
console.log(`http://127.0.0.1:${server.address().port}/watch.html`);
for(const signal of ['SIGINT','SIGTERM'])process.on(signal,()=>server.close(()=>process.exit(0)));

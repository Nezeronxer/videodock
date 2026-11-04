import {BotGuardClient} from './bgutils/core/BotGuardClient.js';
import {WebPoMinter} from './bgutils/core/WebPoMinter.js';
import {parseLooseJSON, getHeaders, buildURL} from './bgutils/utils/helpers.js';

// All errors crossing native IPC are stage names, never page text or response bodies.
Object.assign(globalThis, {videodockAttest: async (videoId: string) => {
  let stage = 'homepage';
  const state = (value: unknown) => { (globalThis as any).__videodockAttestation = value; };
  const boundedText = async (response: Response, maximum: number): Promise<string> => {
    if (!response.ok || !response.body) throw Error();
    const declared = response.headers.get('content-length');
    if (declared && Number(declared) > maximum) throw Error();
    const reader = response.body.getReader();
    const chunks: Uint8Array[] = [];
    let total = 0;
    while (true) {
      const {done, value} = await reader.read();
      if (done) break;
      total += value.length;
      if (total > maximum) { await reader.cancel(); throw Error(); }
      chunks.push(value);
    }
    const bytes = new Uint8Array(total);
    let offset = 0;
    for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
    return new TextDecoder().decode(bytes);
  };
  try {
    const text = await boundedText(await fetch('https://www.youtube.com/', {credentials: 'same-origin'}), 8 * 1024 * 1024);
    const config = text.match(/ytcfg\.set\(({.+?})\);/s)?.[1];
    if (!config) throw Error();
    const cfg = JSON.parse(config);
    const visitor = cfg.VISITOR_DATA || cfg.INNERTUBE_CONTEXT?.client?.visitorData;
    if (typeof visitor !== 'string' || !visitor.length || visitor.length > 4096) throw Error();
    (globalThis as any).yt = {config_: cfg};
    stage = 'challenge';
    const match = text.match(/window\.ytAtN\(\s*({[\s\S]*?})\s*\)/);
    if (!match) throw Error();
    const challenge = parseLooseJSON(match[1]).R?.bgChallenge;
    if (!challenge) throw Error();
    stage = 'interpreter';
    const interpreter = new URL(challenge.interpreterUrl.privateDoNotAccessOrElseTrustedResourceUrlWrappedValue, location.href);
    if (interpreter.protocol !== 'https:' || interpreter.hostname !== 'www.google.com'
        || interpreter.port && interpreter.port !== '443' || interpreter.username || interpreter.password
        || interpreter.search || interpreter.hash || !/^\/js\/th\/[A-Za-z0-9_-]+\.js$/.test(interpreter.pathname)) throw Error();
    await new Promise<void>((resolve) => {
      (globalThis as any).__videodockInterpreterResolve = resolve;
      state({state: 'interpreter', url: interpreter.href});
    });
    state({state: 'pending'});
    stage = 'snapshot';
    const bot = await BotGuardClient.create({program: challenge.program, globalName: challenge.globalName, globalObject: globalThis});
    const signals: any[] = [];
    const snapshot = await bot.snapshot({webPoSignalOutput: signals});
    stage = 'integrity';
    const response = await fetch(buildURL('GenerateIT', true), {method: 'POST', headers: getHeaders(), body: JSON.stringify(['O43z0dpjhgX20SCx4KAo', snapshot])});
    const [integrityToken, estimatedTtlSecs, mintRefreshThreshold, websafeFallbackToken] = JSON.parse(await boundedText(response, 1024 * 1024));
    stage = 'mint';
    const minter = await WebPoMinter.create({integrityToken, estimatedTtlSecs, mintRefreshThreshold, websafeFallbackToken}, signals);
    const content_po = await minter.mintAsWebsafeString(videoId);
    const session_po = await minter.mintAsWebsafeString(visitor);
    state({state: 'ready', content_po, session_po, visitor_data: visitor, user_agent: navigator.userAgent});
  } catch {
    state({state: 'failed', stage});
  }
}});

(() => {
  // crates/desktop/attestation/bgutils/utils/constants.ts
  var GOOG_BASE_URL = "https://jnn-pa.googleapis.com";
  var YT_BASE_URL = "https://www.youtube.com";
  var GOOG_API_KEY = "AIzaSyDyT5W0Jh49F30Pqqtyfdf7pDLFKLJoAnw";
  var USER_AGENT = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36(KHTML, like Gecko)";

  // crates/desktop/attestation/bgutils/utils/helpers.ts
  var base64urlCharRegex = /[-_.]/g;
  var base64urlToBase64Map = {
    "-": "+",
    _: "/",
    ".": "="
  };
  var DeferredPromise = class {
    promise;
    resolve;
    reject;
    constructor() {
      this.promise = new Promise((resolve, reject) => {
        this.resolve = resolve;
        this.reject = reject;
      });
    }
  };
  var BgError = class extends TypeError {
    info;
    constructor(message, info) {
      super(message);
      this.name = "BgError";
      if (info) this.info = info;
    }
  };
  function base64ToU8(base64) {
    let base64Mod;
    if (base64urlCharRegex.test(base64)) {
      base64Mod = base64.replace(base64urlCharRegex, function(match) {
        return base64urlToBase64Map[match];
      });
    } else {
      base64Mod = base64;
    }
    base64Mod = atob(base64Mod);
    return new Uint8Array(
      [...base64Mod].map(
        (char) => char.charCodeAt(0)
      )
    );
  }
  function u8ToBase64(u8, base64url = false) {
    const result = btoa(String.fromCharCode(...u8));
    if (base64url) {
      return result.replace(/\+/g, "-").replace(/\//g, "_");
    }
    return result;
  }
  function parseLooseJSON(looseJson) {
    let jsonStr = looseJson.replace(/,\s*([\]}])/g, "$1");
    jsonStr = jsonStr.replace(/'((?:[^'\\]|\\[\s\S])*)'/g, (_match, innerStr) => {
      const unescaped = innerStr.replace(/\\'/g, "'");
      return JSON.stringify(unescaped);
    });
    jsonStr = jsonStr.replace(/([{,]\s*)([a-zA-Z0-9_$]+)\s*:/g, '$1"$2":');
    const parsedData = JSON.parse(jsonStr);
    const decodeHexEscapes = (value) => {
      return value.replace(/\\x([0-9A-Fa-f]{2})/g, (_match, hex) => {
        return String.fromCharCode(parseInt(hex, 16));
      });
    };
    const normalizeValue = (value) => {
      if (typeof value === "string") {
        const decodedValue = decodeHexEscapes(value);
        const trimmed = decodedValue.trim();
        if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
          try {
            return normalizeValue(JSON.parse(decodedValue));
          } catch {
            return decodedValue;
          }
        }
        return decodedValue;
      }
      if (Array.isArray(value)) {
        return value.map(normalizeValue);
      }
      if (value && typeof value === "object") {
        for (const key in value) {
          value[key] = normalizeValue(value[key]);
        }
      }
      return value;
    };
    return normalizeValue(parsedData);
  }
  function isBrowser() {
    const isBrowser2 = typeof window !== "undefined" && typeof window.document !== "undefined" && typeof window.document.createElement !== "undefined" && typeof window.HTMLElement !== "undefined" && typeof window.navigator !== "undefined" && typeof window.getComputedStyle === "function" && typeof window.requestAnimationFrame === "function" && typeof window.matchMedia === "function";
    const hasValidWindow = Object.getOwnPropertyDescriptor(globalThis, "window")?.get?.toString().includes("[native code]") ?? false;
    return isBrowser2 && hasValidWindow;
  }
  function getHeaders() {
    const headers = {
      "content-type": "application/json+protobuf",
      "x-goog-api-key": GOOG_API_KEY,
      "x-user-agent": "grpc-web-javascript/0.1"
    };
    if (!isBrowser()) {
      headers["user-agent"] = USER_AGENT;
    }
    return headers;
  }
  function buildURL(endpointName, useYouTubeAPI) {
    return `${useYouTubeAPI ? YT_BASE_URL : GOOG_BASE_URL}/${useYouTubeAPI ? "api/jnn/v1" : "$rpc/google.internal.waa.v1.Waa"}/${endpointName}`;
  }

  // crates/desktop/attestation/bgutils/utils/EventEmitterLike.ts
  var EventEmitterLike = class {
    #listeners = /* @__PURE__ */ new Map();
    #onceWrappers = /* @__PURE__ */ new Map();
    emit(type, ...args) {
      const listeners = this.#listeners.get(type);
      if (!listeners || listeners.size === 0)
        return;
      for (const listener of [...listeners]) {
        listener(...args);
      }
    }
    on(type, listener) {
      let listeners = this.#listeners.get(type);
      if (!listeners) {
        listeners = /* @__PURE__ */ new Set();
        this.#listeners.set(type, listeners);
      }
      listeners.add(listener);
    }
    once(type, listener) {
      const wrapper = (...args) => {
        this.off(type, listener);
        listener(...args);
      };
      let wrappersByType = this.#onceWrappers.get(listener);
      if (!wrappersByType) {
        wrappersByType = /* @__PURE__ */ new Map();
        this.#onceWrappers.set(listener, wrappersByType);
      }
      wrappersByType.set(type, wrapper);
      this.on(type, wrapper);
    }
    off(type, listener) {
      const listeners = this.#listeners.get(type);
      if (!listeners)
        return;
      let target = listener;
      const wrappersByType = this.#onceWrappers.get(listener);
      if (wrappersByType) {
        const onceWrapper = wrappersByType.get(type);
        if (onceWrapper) {
          target = onceWrapper;
          wrappersByType.delete(type);
          if (wrappersByType.size === 0)
            this.#onceWrappers.delete(listener);
        }
      }
      listeners.delete(target);
      if (listeners.size === 0)
        this.#listeners.delete(type);
    }
    removeAllListeners(type) {
      if (!type) {
        this.#listeners.clear();
        this.#onceWrappers.clear();
        return;
      }
      this.#listeners.delete(type);
      for (const [listener, wrappersByType] of this.#onceWrappers.entries()) {
        wrappersByType.delete(type);
        if (wrappersByType.size === 0)
          this.#onceWrappers.delete(listener);
      }
    }
  };

  // crates/desktop/attestation/bgutils/core/BotGuardClient.ts
  var BotGuardClient = class _BotGuardClient extends EventEmitterLike {
    vm;
    program;
    userInteractionElement;
    syncSnapshotFunction;
    deferredVmFunctions = new DeferredPromise();
    defaultTimeout = 3e3;
    on(type, listener) {
      super.on(type, listener);
    }
    off(type, listener) {
      super.off(type, listener);
    }
    constructor(options) {
      super();
      if (!options.globalObject || !options.globalName || !options.program) {
        throw new BgError("Invalid options", { options });
      }
      this.userInteractionElement = options.userInteractionElement;
      this.vm = options.globalObject[options.globalName];
      this.program = options.program;
    }
    /**
     * Factory method to create and load a BotGuardClient instance.
     * @param options - Configuration options for the BotGuardClient.
     * @returns A loaded BotGuardClient instance.
     */
    static async create(options) {
      return await new _BotGuardClient(options).load();
    }
    async load() {
      if (!this.vm)
        throw new BgError("EGOU: BotGuard unavailable");
      if (!this.vm.a)
        throw new BgError("ELIU: BotGuard initialization function unavailable");
      const vmSetupCallback = (asyncSnapshotFunction, shutdownFunction, passEventFunction, checkCameraFunction) => {
        this.deferredVmFunctions.resolve({
          asyncSnapshotFunction,
          shutdownFunction,
          passEventFunction,
          checkCameraFunction
        });
      };
      const logEvent = (event, elapsedTime) => {
        this.emit("record-bg-event", { event, elapsedTime });
      };
      const incrementClientErrorCount = (errorCode) => {
        this.emit("increment-client-error-count", { errorCode });
      };
      const recordPayloadSize = (payloadSize) => {
        this.emit("record-payload-size", { payloadSize });
      };
      const recordLatency = (latency, et) => {
        this.emit("record-latency", { latency, et });
      };
      const incrementEventCount = (event) => {
        this.emit("increment-bg-event-count", { event });
      };
      const loggerFunctions = [
        logEvent,
        incrementClientErrorCount,
        recordPayloadSize,
        recordLatency,
        incrementEventCount
      ];
      const vmTelemetryCallback = (latency, eventFlag1, eventFlag2) => {
        let event = "k";
        if (eventFlag1) {
          event = "h";
        } else if (eventFlag2) {
          event = "u";
        }
        incrementEventCount(event);
        logEvent(event, latency);
      };
      try {
        this.syncSnapshotFunction = await this.vm.a(
          this.program,
          vmSetupCallback,
          true,
          this.userInteractionElement,
          vmTelemetryCallback,
          [[], []],
          void 0,
          false,
          loggerFunctions
        )?.[0];
      } catch (error) {
        throw new BgError("Could not load program", { error });
      }
      return this;
    }
    /**
     * Calls a VM function with a timeout.
     * @param vmFunctionName - The name of the VM function to execute.
     * @param timeout - The timeout in milliseconds.
     * @param args - The arguments to pass to the VM function.
     */
    async execute(vmFunctionName, timeout, ...args) {
      return await Promise.race([
        (async () => {
          const vmFunctions = await this.deferredVmFunctions.promise;
          const vmFunction = vmFunctions[vmFunctionName];
          if (!vmFunction)
            throw new BgError(`${vmFunctionName} function not found`);
          return vmFunction(...args);
        })(),
        new Promise(
          (_, reject) => setTimeout(() => reject(new BgError("VM operation timed out")), timeout)
        )
      ]);
    }
    /**
     * Takes a snapshot asynchronously.
     * @returns The snapshot result.
     * @example
     * ```ts
     * const result = await botguard.snapshot({
     *   contentBinding: {
     *     c: "a=6&a2=10&b=SZWDwKVIuixOp7Y4euGTgwckbJA&c=1729143849&d=1&t=7200&c1a=1&c6a=1&c6b=1&hh=HrMb5mRWTyxGJphDr0nW2Oxonh0_wl2BDqWuLHyeKLo",
     *     e: "ENGAGEMENT_TYPE_VIDEO_LIKE",
     *     encryptedVideoId: "P-vC09ZJcnM"
     *    }
     * });
     *
     * console.log(result);
     * ```
     */
    async snapshot(args, timeout = this.defaultTimeout) {
      return await new Promise(async (resolve, reject) => {
        await this.execute(
          "asyncSnapshotFunction",
          timeout,
          (response) => resolve(response),
          [
            args.contentBinding,
            args.signedTimestamp,
            args.webPoSignalOutput,
            args.skipPrivacyBuffer
          ]
        ).catch(reject);
      });
    }
    /**
     * Passes an event to the VM.
     */
    async passEvent(args, timeout = this.defaultTimeout) {
      return this.execute("passEventFunction", timeout, args);
    }
    /**
     * Checks the "camera".
     */
    async checkCamera(args, timeout = this.defaultTimeout) {
      return this.execute("checkCameraFunction", timeout, args);
    }
    /**
     * Shuts down the VM. Once called, the VM is no longer usable.
     */
    async shutdown(timeout = this.defaultTimeout) {
      return this.execute("shutdownFunction", timeout);
    }
    /**
     * Takes a snapshot synchronously.
     * @returns The snapshot result.
     */
    async snapshotSynchronous(args) {
      if (!this.syncSnapshotFunction)
        throw new BgError("Synchronous snapshot function not found");
      return this.syncSnapshotFunction([
        args.contentBinding,
        args.signedTimestamp,
        args.webPoSignalOutput,
        args.skipPrivacyBuffer
      ]);
    }
  };

  // crates/desktop/attestation/bgutils/core/WebPoMinter.ts
  var WebPoMinter = class _WebPoMinter {
    mintCallback;
    constructor(mintCallback) {
      this.mintCallback = mintCallback;
    }
    /**
     * Factory method to create a WebPoMinter instance.
     * @param integrityTokenResponse - The integrity token response object.
     * @param webPoSignalOutput - The output array containing the minter function.
     */
    static async create(integrityTokenResponse, webPoSignalOutput) {
      const getMinter = webPoSignalOutput[0];
      if (!getMinter)
        throw new BgError("PMD:Undefined");
      if (!integrityTokenResponse.integrityToken)
        throw new BgError("No integrity token provided", { integrityTokenResponse });
      const mintCallback = await getMinter(base64ToU8(integrityTokenResponse.integrityToken));
      if (!(mintCallback instanceof Function))
        throw new BgError("APF:Failed");
      return new _WebPoMinter(mintCallback);
    }
    /**
     * Mints a proof and returns it as a web-safe base64 string.
     * @param contentBinding - A Visitor ID, Video ID, or Data Sync ID.
     */
    async mintAsWebsafeString(contentBinding) {
      return u8ToBase64(await this.mint(contentBinding), true);
    }
    /**
     * Mints a proof and returns it as a Uint8Array.
     * @param contentBinding - A Visitor ID, Video ID, or Data Sync ID.
     */
    async mint(contentBinding) {
      const result = await this.mintCallback(new TextEncoder().encode(contentBinding));
      if (!result)
        throw new BgError("YNJ:Undefined");
      if (!(result instanceof Uint8Array))
        throw new BgError("ODM:Invalid");
      return result;
    }
  };

  // crates/desktop/attestation/entry.ts
  Object.assign(globalThis, { videodockAttest: async (videoId) => {
    let stage = "homepage";
    const state = (value) => {
      globalThis.__videodockAttestation = value;
    };
    const boundedText = async (response, maximum) => {
      if (!response.ok || !response.body) throw Error();
      const declared = response.headers.get("content-length");
      if (declared && Number(declared) > maximum) throw Error();
      const reader = response.body.getReader();
      const chunks = [];
      let total = 0;
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        total += value.length;
        if (total > maximum) {
          await reader.cancel();
          throw Error();
        }
        chunks.push(value);
      }
      const bytes = new Uint8Array(total);
      let offset = 0;
      for (const chunk of chunks) {
        bytes.set(chunk, offset);
        offset += chunk.length;
      }
      return new TextDecoder().decode(bytes);
    };
    try {
      const text = await boundedText(await fetch("https://www.youtube.com/", { credentials: "same-origin" }), 8 * 1024 * 1024);
      const config = text.match(/ytcfg\.set\(({.+?})\);/s)?.[1];
      if (!config) throw Error();
      const cfg = JSON.parse(config);
      const visitor = cfg.VISITOR_DATA || cfg.INNERTUBE_CONTEXT?.client?.visitorData;
      if (typeof visitor !== "string" || !visitor.length || visitor.length > 4096) throw Error();
      globalThis.yt = { config_: cfg };
      stage = "challenge";
      const match = text.match(/window\.ytAtN\(\s*({[\s\S]*?})\s*\)/);
      if (!match) throw Error();
      const challenge = parseLooseJSON(match[1]).R?.bgChallenge;
      if (!challenge) throw Error();
      stage = "interpreter";
      const interpreter = new URL(challenge.interpreterUrl.privateDoNotAccessOrElseTrustedResourceUrlWrappedValue, location.href);
      if (interpreter.protocol !== "https:" || interpreter.hostname !== "www.google.com" || interpreter.port && interpreter.port !== "443" || interpreter.username || interpreter.password || interpreter.search || interpreter.hash || !/^\/js\/th\/[A-Za-z0-9_-]+\.js$/.test(interpreter.pathname)) throw Error();
      await new Promise((resolve) => {
        globalThis.__videodockInterpreterResolve = resolve;
        state({ state: "interpreter", url: interpreter.href });
      });
      state({ state: "pending" });
      stage = "snapshot";
      const bot = await BotGuardClient.create({ program: challenge.program, globalName: challenge.globalName, globalObject: globalThis });
      const signals = [];
      const snapshot = await bot.snapshot({ webPoSignalOutput: signals });
      stage = "integrity";
      const response = await fetch(buildURL("GenerateIT", true), { method: "POST", headers: getHeaders(), body: JSON.stringify(["O43z0dpjhgX20SCx4KAo", snapshot]) });
      const [integrityToken, estimatedTtlSecs, mintRefreshThreshold, websafeFallbackToken] = JSON.parse(await boundedText(response, 1024 * 1024));
      stage = "mint";
      const minter = await WebPoMinter.create({ integrityToken, estimatedTtlSecs, mintRefreshThreshold, websafeFallbackToken }, signals);
      const content_po = await minter.mintAsWebsafeString(videoId);
      const session_po = await minter.mintAsWebsafeString(visitor);
      state({ state: "ready", content_po, session_po, visitor_data: visitor, user_agent: navigator.userAgent });
    } catch {
      state({ state: "failed", stage });
    }
  } });
})();

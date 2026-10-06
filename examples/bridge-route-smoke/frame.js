(() => {
  "use strict";
  (async () => {
    try {
      const title = await Niva.window.title();
      parent.__bridgeRouteReceiveFrameResult({
        ok: true,
        title,
        href: location.href,
        frame: window,
        sessionSetAvailable: typeof window.__bridgeRouteSessionValues === "function",
        sessionCount: window.__bridgeRouteSessionCount(),
        events: window.__bridgeRouteEvents.slice()
      });
    } catch (error) {
      parent.__bridgeRouteReceiveFrameResult({
        ok: false,
        message: String(error && error.message || error),
        stack: String(error && error.stack || ""),
        href: location.href,
        frame: window,
        events: window.__bridgeRouteEvents || []
      });
    }
  })();
})();

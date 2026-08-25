// auctioning.lol — Phantom wallet helper.
// Loaded before the WASM bundle; provides a tiny promise-based facade and
// detects the provider without hard failures when Phantom is absent.
(function () {
  "use strict";

  function provider() {
    return (window.phantom && window.phantom.solana) || window.solana || null;
  }

  window.auctioning = {
    hasPhantom: function () {
      var p = provider();
      return !!(p && p.isPhantom);
    },

    phantomUrl: "https://phantom.app/",

    connect: function () {
      var p = provider();
      if (!p) return Promise.reject(new Error("phantom-not-found"));
      return p.connect().then(function (res) {
        if (!res || !res.publicKey) throw new Error("no-public-key");
        return res.publicKey.toString();
      });
    },

    disconnect: function () {
      var p = provider();
      return p ? p.disconnect() : Promise.resolve();
    },

    signMessageUtf8: function (message) {
      var p = provider();
      if (!p) return Promise.reject(new Error("phantom-not-found"));
      var bytes = new TextEncoder().encode(message);
      return p.signMessage(bytes, "utf8");
    },
  };

  // Reconnect silently on load when previously authorized.
  window.addEventListener("load", function () {
    var p = provider();
    if (p && typeof p.isConnected === "boolean" && p.isConnected && p.publicKey) {
      // The Leptos app polls this flag on startup.
      window.auctioning.eagerPublicKey = p.publicKey.toString();
    }
  });
})();

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

    // Base58 (Bitcoin alphabet) for ed25519 signatures returned by Phantom.
    base58: function (bytes) {
      var ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
      var digits = [0];
      for (var i = 0; i < bytes.length; i++) {
        var carry = bytes[i];
        for (var j = 0; j < digits.length; j++) {
          carry += digits[j] << 8;
          digits[j] = carry % 58;
          carry = (carry / 58) | 0;
        }
        while (carry > 0) {
          digits.push(carry % 58);
          carry = (carry / 58) | 0;
        }
      }
      var out = "";
      for (var k = 0; k < bytes.length && bytes[k] === 0; k++) out += ALPHABET[0];
      for (var d = digits.length - 1; d >= 0; d--) out += ALPHABET[digits[d]];
      return out;
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

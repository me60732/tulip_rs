# Live Demo

An interactive browser demo — indicators computed in WebAssembly via [`tulip-rs-wasm`](https://www.npmjs.com/package/tulip-rs-wasm), chart rendered with [Lightweight Charts v5](https://www.tradingview.com/lightweight-charts/).

<div style="margin: 1rem 0; text-align: right;">
  <a href="../demo.html" target="_blank" style="font-size: 0.8rem; color: var(--md-accent-fg-color);">↗ Open full screen</a>
</div>

<iframe
  src="../demo.html"
  style="width: 100%; height: 680px; border: none; border-radius: 8px; display: block;"
  allow="cross-origin-isolated"
></iframe>

---

## What you can do

- Click **＋ Add Indicator** to open the dialog and choose any of the 70+ supported indicators
- Set numeric options (period, etc.) and optional output visibility before adding
- Each active indicator appears as a badge — click **✕** to remove it
- Use the **1Y / 2Y / 5Y / All** zoom buttons in the header to change the visible range

## Notes

- The WASM engine is loaded on first visit from the jsDelivr CDN — subsequent loads are cached by the browser.
- Data is MSFT daily OHLCV (static, bundled with the demo).
- Built with [`tulip-rs-lwc`](https://github.com/me60732/tulip-rs-lwc) — the Lightweight Charts plugin for tulip-rs.

(function () {
    "use strict";

    function attach(wrap) {
        if (wrap.dataset.floatBar) return;
        if (!wrap.querySelector("table")) return;

        /* Mark the wrapper so CSS hides its native scrollbar */
        wrap.dataset.floatBar = "1";

        /* Create the fixed floating bar */
        var bar = document.createElement("div");
        bar.className = "table-float-scrollbar";
        bar.setAttribute("aria-hidden", "true");
        var spacer = document.createElement("div");
        bar.appendChild(spacer);
        document.body.appendChild(bar);

        /* Position the bar and toggle visibility */
        function update() {
            var rect = wrap.getBoundingClientRect();
            var overflows = wrap.scrollWidth > wrap.clientWidth;
            var inView = rect.top < window.innerHeight && rect.bottom > 14;

            if (!overflows || !inView) {
                bar.style.display = "none";
                return;
            }

            /* Explicitly set 'block' — empty string would fall back to CSS 'none' */
            bar.style.display = "block";
            bar.style.left = rect.left + "px";
            bar.style.width = rect.width + "px";
            spacer.style.width = wrap.scrollWidth + "px";

            /* Align thumb without triggering the sync loop */
            if (bar.scrollLeft !== wrap.scrollLeft) {
                bar.scrollLeft = wrap.scrollLeft;
            }
        }

        /* Bidirectional sync — flags cleared after a rAF tick so the echoed
       scroll event (fired async by the browser) is ignored before they reset */
        var barBusy = false;
        var wrapBusy = false;

        bar.addEventListener(
            "scroll",
            function () {
                if (wrapBusy) return;
                barBusy = true;
                wrap.scrollLeft = bar.scrollLeft;
                requestAnimationFrame(function () {
                    barBusy = false;
                });
            },
            { passive: true },
        );

        wrap.addEventListener(
            "scroll",
            function () {
                if (barBusy) return;
                wrapBusy = true;
                if (bar.scrollLeft !== wrap.scrollLeft) {
                    bar.scrollLeft = wrap.scrollLeft;
                }
                requestAnimationFrame(function () {
                    wrapBusy = false;
                });
            },
            { passive: true },
        );

        window.addEventListener("scroll", update, { passive: true });
        window.addEventListener("resize", update, { passive: true });
        if (window.ResizeObserver) {
            new ResizeObserver(update).observe(wrap);
        }

        update();
    }

    function setup() {
        document.querySelectorAll(".md-typeset__scrollwrap").forEach(attach);
    }

    document.readyState === "loading"
        ? document.addEventListener("DOMContentLoaded", setup)
        : setup();

    /* MkDocs Material instant navigation */
    document.addEventListener("DOMContentSwitch", setup);
})();

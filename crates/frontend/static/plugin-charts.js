/*!
 * PluginCharts — тонкая тема-aware обёртка над Chart.js для плагинов-графиков.
 *
 * Грузится в iframe плагина ПОСЛЕ vendor/chartjs/chart.umd.min.js (window.Chart).
 * Плагин в client_script зовёт:
 *
 *   const rows = await host.invoke("data", {});
 *   PluginCharts.render(root, spec, rows);
 *
 * spec (компактная форма, удобная для LLM):
 *   line/area/bar/stacked-bar:
 *     { type, title, x:"date", series:[{ y:"revenue", label:"Выручка" }],
 *       stacked:false, horizontal:false, format:"money", alternatives:["bar","area"] }
 *   pie/doughnut:
 *     { type, title, category:"marketplace", value:"revenue", format:"money" }
 *
 * Цвета осей/сетки/текста берутся из CSS-переменных активной темы приложения,
 * поэтому график автоматически совпадает со светлой/тёмной темой. Палитра серий —
 * фиксированная, читаемая на обеих темах. PluginCharts.applyTheme() перерисовывает
 * все живые графики при смене темы (вызывается из bootstrap iframe).
 */
(function () {
  "use strict";

  if (window.PluginCharts) return;

  // Контроллеры живых графиков: { rebuild(), destroy() }.
  var REGISTRY = new Set();
  var STYLE_ID = "pcharts-style";

  function injectStyleOnce() {
    if (document.getElementById(STYLE_ID)) return;
    var css =
      ".pcharts{display:flex;flex-direction:column;gap:10px;min-height:0;height:100%;}" +
      ".pcharts__head{display:flex;align-items:center;justify-content:space-between;gap:12px;flex-wrap:wrap;}" +
      ".pcharts__title{font-weight:600;font-size:15px;color:var(--color-text-primary,#e5e7eb);}" +
      ".pcharts__chips{display:flex;gap:6px;flex-wrap:wrap;}" +
      ".pcharts__chip{cursor:pointer;border:1px solid var(--color-border,rgba(128,128,128,.4));" +
      "background:transparent;color:var(--color-text-secondary,#9ca3af);border-radius:999px;" +
      "padding:3px 12px;font-size:12px;line-height:1.4;transition:all .12s;}" +
      ".pcharts__chip:hover{color:var(--color-text-primary,#e5e7eb);" +
      "border-color:var(--color-primary,#4f8cff);}" +
      ".pcharts__chip--active{background:var(--color-primary,#4f8cff);" +
      "border-color:var(--color-primary,#4f8cff);color:#fff;}" +
      ".pcharts__canvas{position:relative;flex:1 1 auto;min-height:260px;}" +
      ".pcharts__empty{padding:24px;color:var(--color-text-secondary,#9ca3af);" +
      "text-align:center;font-size:14px;}";
    var el = document.createElement("style");
    el.id = STYLE_ID;
    el.textContent = css;
    document.head.appendChild(el);
  }

  function cssVar(name, fallback) {
    try {
      var v = getComputedStyle(document.documentElement)
        .getPropertyValue(name)
        .trim();
      return v || fallback;
    } catch (e) {
      return fallback;
    }
  }

  function themeColors() {
    return {
      text: cssVar("--color-text-primary", "#e5e7eb"),
      muted: cssVar("--color-text-secondary", "#9ca3af"),
      grid: cssVar("--color-border-light", cssVar("--color-border", "rgba(128,128,128,.2)")),
      tooltipBg: cssVar("--color-surface", "#1f2937"),
    };
  }

  // Палитра, читаемая на светлой и тёмной теме.
  var PALETTE = [
    "#4f8cff", "#22c55e", "#f59e0b", "#ef4444", "#a855f7",
    "#06b6d4", "#ec4899", "#84cc16", "#f97316", "#14b8a6",
  ];

  function colorAt(i) {
    return PALETTE[i % PALETTE.length];
  }

  function withAlpha(hex, alpha) {
    var h = hex.replace("#", "");
    if (h.length === 3) h = h[0] + h[0] + h[1] + h[1] + h[2] + h[2];
    var n = parseInt(h, 16);
    if (isNaN(n)) return hex;
    var r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
    return "rgba(" + r + "," + g + "," + b + "," + alpha + ")";
  }

  function formatter(format) {
    var ru = function (opts) {
      try {
        return new Intl.NumberFormat("ru-RU", opts);
      } catch (e) {
        return { format: function (v) { return String(v); } };
      }
    };
    if (format === "money") {
      var m = ru({ maximumFractionDigits: 0 });
      return function (v) { return m.format(v) + " ₽"; };
    }
    if (format === "percent") {
      var p = ru({ maximumFractionDigits: 1 });
      return function (v) { return p.format(v) + "%"; };
    }
    if (format === "int") {
      var i = ru({ maximumFractionDigits: 0 });
      return function (v) { return i.format(v); };
    }
    var d = ru({ maximumFractionDigits: 2 });
    return function (v) { return d.format(v); };
  }

  function num(value) {
    if (value === null || value === undefined || value === "") return null;
    var n = typeof value === "number" ? value : parseFloat(String(value).replace(",", "."));
    return isNaN(n) ? null : n;
  }

  function isPie(type) {
    return type === "pie" || type === "doughnut";
  }

  // ── Сборка Chart.js-конфига из spec + строк ────────────────────────────────
  function buildConfig(type, spec, rows, colors) {
    var fmt = formatter(spec.format);
    rows = Array.isArray(rows) ? rows : [];

    if (isPie(type)) {
      var labels = rows.map(function (r) { return String(r[spec.category]); });
      var values = rows.map(function (r) { return num(r[spec.value]); });
      return {
        type: type,
        data: {
          labels: labels,
          datasets: [{
            data: values,
            backgroundColor: labels.map(function (_, i) { return colorAt(i); }),
            borderColor: colors.tooltipBg,
            borderWidth: 2,
          }],
        },
        options: pieOptions(spec, colors, fmt),
      };
    }

    // Картезианские типы.
    var baseType = type === "area" ? "line" : type;
    var fill = type === "area";
    var stacked = !!spec.stacked || type === "stacked-bar";
    if (type === "stacked-bar") baseType = "bar";
    var horizontal = !!spec.horizontal;

    var xLabels = rows.map(function (r) { return String(r[spec.x]); });
    var series = Array.isArray(spec.series) && spec.series.length
      ? spec.series
      : [{ y: spec.y, label: spec.label || spec.y }];

    var datasets = series.map(function (s, i) {
      var c = colorAt(i);
      return {
        label: s.label || s.y,
        data: rows.map(function (r) { return num(r[s.y]); }),
        borderColor: c,
        backgroundColor: baseType === "line" ? withAlpha(c, fill ? 0.25 : 1) : withAlpha(c, 0.85),
        fill: baseType === "line" ? fill : undefined,
        tension: 0.25,
        borderWidth: 2,
        pointRadius: baseType === "line" ? 2 : 0,
        maxBarThickness: 48,
      };
    });

    return {
      type: baseType,
      data: { labels: xLabels, datasets: datasets },
      options: cartesianOptions(spec, colors, fmt, stacked, horizontal),
    };
  }

  function legendBlock(colors, show) {
    return {
      display: show,
      labels: { color: colors.text, usePointStyle: true, boxWidth: 8 },
    };
  }

  function tooltipBlock(colors, fmt, isPercentShare) {
    return {
      backgroundColor: colors.tooltipBg,
      titleColor: colors.text,
      bodyColor: colors.text,
      borderColor: colors.grid,
      borderWidth: 1,
      callbacks: {
        label: function (ctx) {
          var v = ctx.parsed && ctx.parsed.y !== undefined ? ctx.parsed.y : ctx.parsed;
          var name = ctx.dataset.label || ctx.label || "";
          var base = (name ? name + ": " : "") + fmt(v);
          if (isPercentShare) {
            var total = ctx.dataset.data.reduce(function (a, b) { return a + (num(b) || 0); }, 0);
            if (total > 0) base += " (" + (v / total * 100).toFixed(1) + "%)";
          }
          return base;
        },
      },
    };
  }

  function cartesianOptions(spec, colors, fmt, stacked, horizontal) {
    var valueAxis = {
      stacked: stacked,
      ticks: { color: colors.muted, callback: function (v) { return fmt(v); } },
      grid: { color: colors.grid },
    };
    var catAxis = {
      stacked: stacked,
      ticks: { color: colors.muted, autoSkip: true, maxRotation: 0 },
      grid: { display: false },
    };
    var scales = horizontal ? { x: valueAxis, y: catAxis } : { x: catAxis, y: valueAxis };
    return {
      responsive: true,
      maintainAspectRatio: false,
      indexAxis: horizontal ? "y" : "x",
      interaction: { mode: "index", intersect: false },
      plugins: {
        legend: legendBlock(colors, (spec.series || []).length > 1),
        tooltip: tooltipBlock(colors, fmt, false),
      },
      scales: scales,
    };
  }

  function pieOptions(spec, colors, fmt) {
    return {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: legendBlock(colors, true),
        tooltip: tooltipBlock(colors, fmt, true),
      },
    };
  }

  // ── Публичный render ───────────────────────────────────────────────────────
  function render(target, spec, rows) {
    if (typeof Chart === "undefined") {
      throw new Error("Chart.js не загружен (window.Chart отсутствует)");
    }
    injectStyleOnce();
    spec = spec || {};
    var defaultType = spec.type || "line";

    // Контейнер: либо переданный root, либо сам canvas.
    var root = target;
    if (root && root.tagName === "CANVAS") {
      // Уже canvas — рисуем прямо в него, без головы/чипов.
      return mountChart(root, defaultType, spec, rows, null);
    }

    root.classList.add("pcharts");
    root.replaceChildren();

    var head = document.createElement("div");
    head.className = "pcharts__head";
    var title = document.createElement("div");
    title.className = "pcharts__title";
    title.textContent = spec.title || "";
    head.appendChild(title);

    var chips = document.createElement("div");
    chips.className = "pcharts__chips";
    head.appendChild(chips);
    root.appendChild(head);

    var wrap = document.createElement("div");
    wrap.className = "pcharts__canvas";
    var canvas = document.createElement("canvas");
    wrap.appendChild(canvas);
    root.appendChild(wrap);

    if (!Array.isArray(rows) || rows.length === 0) {
      wrap.replaceChildren();
      var empty = document.createElement("div");
      empty.className = "pcharts__empty";
      empty.textContent = "Нет данных для отображения";
      wrap.appendChild(empty);
      return { rebuild: function () {}, destroy: function () {} };
    }

    var controller = mountChart(canvas, defaultType, spec, rows, function (newType) {
      renderChips(chips, newType);
    });

    // Чипы-переключатели типов: default + альтернативы.
    var alts = [defaultType].concat(
      (spec.alternatives || []).filter(function (t) { return t !== defaultType; })
    );

    function renderChips(container, activeType) {
      container.replaceChildren();
      if (alts.length < 2) return;
      alts.forEach(function (t) {
        var chip = document.createElement("button");
        chip.type = "button";
        chip.className = "pcharts__chip" + (t === activeType ? " pcharts__chip--active" : "");
        chip.textContent = typeLabel(t);
        chip.addEventListener("click", function () {
          controller.setType(t);
          renderChips(container, t);
        });
        container.appendChild(chip);
      });
    }
    renderChips(chips, defaultType);

    return controller;
  }

  function typeLabel(t) {
    var map = {
      line: "Линия", area: "Область", bar: "Столбцы",
      "stacked-bar": "С накоплением", pie: "Круговая", doughnut: "Кольцо",
    };
    return map[t] || t;
  }

  // Создаёт Chart на canvas и регистрирует контроллер для смены типа/темы.
  function mountChart(canvas, type, spec, rows, onTypeChange) {
    var state = { type: type };
    var chart = null;

    function build() {
      if (chart) { chart.destroy(); chart = null; }
      var cfg = buildConfig(state.type, spec, rows, themeColors());
      chart = new Chart(canvas.getContext("2d"), cfg);
    }
    build();

    var controller = {
      setType: function (t) {
        state.type = t;
        build();
        if (onTypeChange) onTypeChange(t);
      },
      rebuild: build, // перечитывает тему
      destroy: function () {
        if (chart) chart.destroy();
        chart = null;
        REGISTRY.delete(controller);
      },
    };
    REGISTRY.add(controller);
    return controller;
  }

  // Перерисовать все живые графики (смена темы приложения).
  function applyTheme() {
    REGISTRY.forEach(function (c) {
      try { c.rebuild(); } catch (e) { /* график мог быть размонтирован */ }
    });
  }

  window.PluginCharts = {
    render: render,
    applyTheme: applyTheme,
    palette: PALETTE.slice(),
  };
})();

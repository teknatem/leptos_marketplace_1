function formatNumber(value, fractionDigits) {
  const parsed = Number(value);
  const safe = Number.isFinite(parsed) ? parsed : 0;
  const parts = Math.abs(safe).toFixed(fractionDigits).split(".");
  const whole = parts[0].replace(/\B(?=(\d{3})+(?!\d))/g, "\u00a0");
  const sign = safe < 0 ? "−" : "";
  return parts.length === 2 ? `${sign}${whole},${parts[1]}` : `${sign}${whole}`;
}

const intFmt = { format(value) { return formatNumber(value, 0); } };
const statusNames = {
  DELIVERED: "Доставлен",
  CANCELLED: "Отменён",
  IN_DELIVERY: "В доставке",
  PROCESSING: "Обрабатывается"
};

function money(value) {
  return `${formatNumber(value, 2)} ₽`;
}

function humanDate(value) {
  if (!value || value.length < 10) return "—";
  return `${value.slice(8, 10)}.${value.slice(5, 7)}.${value.slice(0, 4)}`;
}

function setText(element, value) {
  element.textContent = value == null ? "" : String(value);
}

function td(value, className = "") {
  const cell = document.createElement("td");
  if (className) cell.className = className;
  setText(cell, value);
  return cell;
}

function monthPeriod(month) {
  const parts = String(month || "").split("-");
  const year = Number(parts[0]);
  const monthIndex = Number(parts[1]);
  if (!year || !monthIndex) throw new Error("Выберите месяц");
  const lastDay = new Date(year, monthIndex, 0).getDate();
  return {
    dateFrom: `${year}-${String(monthIndex).padStart(2, "0")}-01`,
    dateTo: `${year}-${String(monthIndex).padStart(2, "0")}-${String(lastDay).padStart(2, "0")}`,
    days: lastDay
  };
}

function currentMonth() {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}`;
}

function csvCell(value) {
  let text = value == null ? "" : String(value);
  if (/^[=+\-@]/.test(text)) text = `'${text}`;
  return `"${text.replace(/"/g, '""')}"`;
}

function themeColor(name, fallback) {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

export async function mount(root, host) {
  const state = {
    chart: null,
    page: 1,
    pageSize: 50,
    totalRows: 0,
    selectedDay: "",
    sortField: "order_date",
    sortDirection: "desc",
    loading: false,
    exporting: false,
    lastOrders: null
  };

  root.innerHTML = `
    <main class="ym-payments">
      <header class="ym-head">
        <div>
          <h1>Фактические выплаты Яндекс Маркета</h1>
          <p>Состояние заказов, обязательства Яндекса и фактически распределённые выплаты</p>
        </div>
        <div id="freshness" class="freshness"></div>
      </header>

      <section class="filters" aria-label="Фильтры отчёта">
        <label class="field">Месяц
          <input id="month" type="month">
        </label>
        <label class="field field--cabinet">Кабинет Яндекс
          <select id="business-id"><option value="">Все кабинеты</option></select>
        </label>
        <button id="refresh" class="btn btn--primary" type="button">Сформировать</button>
      </section>

      <div id="error" class="alert alert--error is-hidden" role="alert"></div>
      <div id="status" class="status"></div>

      <nav class="tabs" aria-label="Разделы отчёта">
        <button class="tab tab--active" data-tab="summary" type="button">Итоги</button>
        <button class="tab" data-tab="chart" type="button">График</button>
        <button class="tab" data-tab="orders" type="button">По заказам</button>
        <button class="tab" data-tab="costs" type="button">Начисления и удержания</button>
      </nav>

      <section id="summary-pane" class="pane" data-pane="summary">
        <section class="cards cards--compact" aria-label="Ключевые итоги">
          <article class="card card--primary"><span>Фактически получено YM по всем заказам когорты</span><strong id="total-bank">—</strong></article>
          <article class="card card--warning"><span>Ожидает удержания</span><strong id="total-pending">—</strong></article>
        </section>

      <section class="summary-grid">
        <article class="panel compact-panel">
          <div class="panel-head"><div><h2>Текущее состояние заказов</h2><p>Заказы, созданные в выбранном месяце</p></div></div>
          <table class="mini-table">
            <thead><tr><th>Состояние</th><th class="num">Количество</th><th class="num">Сумма</th></tr></thead>
            <tbody id="state-body"></tbody>
          </table>
        </article>
        <article class="panel compact-panel">
          <div class="panel-head"><div><h2>Расчёты с Яндекс Маркетом</h2><p>Доставленные заказы против фактических выплат месяца</p></div></div>
          <table class="mini-table">
            <thead><tr><th>Расчёт</th><th class="num">Заказов</th><th class="num">Сумма</th></tr></thead>
            <tbody id="settlement-body"></tbody>
          </table>
        </article>
      </section>

      <section class="panel compact-panel">
        <div class="panel-head">
          <div>
            <h2>Начисления и удержания</h2>
            <p id="financial-caption">Доля рассчитывается от валовой суммы реализации</p>
          </div>
        </div>
        <table class="mini-table financial-table">
          <thead><tr><th>Вид расчёта</th><th class="num">Сумма</th><th class="num">% от реализации</th></tr></thead>
          <tbody id="financial-body"></tbody>
        </table>
      </section>
      </section>

      <section id="chart-pane" class="panel chart-panel pane is-hidden" data-pane="chart">
        <div class="panel-head">
          <div>
            <h2>Результат заказов по дням месяца</h2>
            <p>Все последующие события отнесены к московской дате создания заказа</p>
          </div>
          <button id="clear-day" class="btn btn--ghost is-hidden" type="button">Сбросить день</button>
        </div>
        <div class="chart-wrap"><canvas id="payments-chart"></canvas></div>
      </section>

      <section id="orders-pane" class="panel pane is-hidden" data-pane="orders">
        <div class="panel-head">
          <div><h2>Фактический результат по каждому заказу</h2><p id="orders-caption">Все заказы выбранного месяца</p></div>
          <div class="table-actions">
            <button id="export-csv" class="btn btn--ghost" type="button">Выгрузить CSV</button>
            <label class="page-size">Строк
              <select id="page-size"><option value="20">20</option><option value="50" selected>50</option><option value="100">100</option><option value="200">200</option></select>
            </label>
          </div>
        </div>
        <div class="table-wrap">
          <table class="data-table">
            <thead><tr>
              <th data-sort="order_date">Дата заказа, МСК</th>
              <th data-sort="order_id">Заказ</th>
              <th data-sort="order_status">Статус</th>
              <th class="num" data-sort="order_amount">Цена заказа</th>
              <th data-sort="realization_date">Реализация</th>
              <th class="num">Сумма реализации</th>
              <th class="num">Возврат</th>
              <th data-sort="payment_date">Последняя оплата YM</th>
              <th data-sort="bank_order_id">Поручения</th>
              <th data-sort="cabinet_name">Кабинет</th>
              <th class="num" data-sort="direct_net">Прямое нетто</th>
              <th class="num" data-sort="allocated_shared">Общие расходы</th>
              <th class="num" data-sort="final_payment">Получено от YM</th>
            </tr></thead>
            <tbody id="orders-body"></tbody>
          </table>
        </div>
        <footer class="pager">
          <span id="page-info">—</span>
          <div><button id="prev-page" class="btn btn--ghost" type="button">← Назад</button><button id="next-page" class="btn btn--ghost" type="button">Вперёд →</button></div>
        </footer>
      </section>

      <section id="costs-pane" class="panel pane is-hidden" data-pane="costs">
        <div class="panel-head"><div><h2>Общие начисления и удержания без заказов</h2><p>Строки p907 выбранного месяца без номера заказа и SKU</p></div></div>
        <div class="table-wrap">
          <table class="data-table">
            <thead><tr><th>Начислено</th><th>Акт / период</th><th>Удержано</th><th>Поручение</th><th>Кабинет</th><th>Источник / услуга</th><th>Статус</th><th class="num">Сумма</th></tr></thead>
            <tbody id="costs-body"></tbody>
          </table>
        </div>
      </section>
    </main>`;

  const monthEl = root.querySelector("#month");
  const businessEl = root.querySelector("#business-id");
  const refreshEl = root.querySelector("#refresh");
  const statusEl = root.querySelector("#status");
  const errorEl = root.querySelector("#error");
  const freshnessEl = root.querySelector("#freshness");
  const stateBody = root.querySelector("#state-body");
  const settlementBody = root.querySelector("#settlement-body");
  const financialBody = root.querySelector("#financial-body");
  const financialCaption = root.querySelector("#financial-caption");
  const ordersBody = root.querySelector("#orders-body");
  const costsBody = root.querySelector("#costs-body");
  const pageInfoEl = root.querySelector("#page-info");
  const prevEl = root.querySelector("#prev-page");
  const nextEl = root.querySelector("#next-page");
  const pageSizeEl = root.querySelector("#page-size");
  const exportEl = root.querySelector("#export-csv");
  const chartCanvas = root.querySelector("#payments-chart");
  const clearDayEl = root.querySelector("#clear-day");
  const captionEl = root.querySelector("#orders-caption");
  monthEl.value = currentMonth();

  function activatePane(name) {
    for (const pane of root.querySelectorAll(".pane[data-pane]")) {
      pane.classList.toggle("is-hidden", pane.dataset.pane !== name);
    }
    for (const item of root.querySelectorAll(".tab")) {
      const active = item.dataset.tab === name;
      item.classList.toggle("tab--active", active);
      item.setAttribute("aria-selected", String(active));
    }
    if (name === "chart" && state.chart) {
      setTimeout(() => state.chart && state.chart.resize(), 0);
    }
  }

  function args() {
    const period = monthPeriod(monthEl.value);
    return { dateFrom: period.dateFrom, dateTo: period.dateTo, businessId: businessEl.value };
  }

  function orderArgs(page, pageSize) {
    return {
      ...args(),
      paymentDay: state.selectedDay,
      page,
      pageSize,
      sortField: state.sortField,
      sortDirection: state.sortDirection
    };
  }

  function showError(error) {
    errorEl.textContent = error && error.message ? error.message : String(error);
    errorEl.classList.remove("is-hidden");
  }

  function clearError() {
    errorEl.classList.add("is-hidden");
    errorEl.textContent = "";
  }

  function setLoading(value, message = "Загрузка отчёта…") {
    state.loading = value;
    refreshEl.disabled = value;
    prevEl.disabled = value || state.page <= 1;
    nextEl.disabled = value;
    exportEl.disabled = value || state.exporting;
    statusEl.textContent = value ? message : "";
  }

  function renderFreshness(data) {
    const parts = [];
    if (data.loaded_at_utc) parts.push(`Загружено: ${humanDate(data.loaded_at_utc)}`);
    if (data.last_transaction_date) parts.push(`Последняя операция: ${humanDate(data.last_transaction_date)}`);
    if (data.last_bank_order_date) parts.push(`Последнее поручение: ${humanDate(data.last_bank_order_date)}`);
    freshnessEl.textContent = parts.length ? parts.join(" · ") : "Данные p907 ещё не загружены";
  }

  function renderCards(summary) {
    setText(root.querySelector("#total-bank"), money(summary.totals.bank_sum));
    setText(root.querySelector("#total-pending"), money(summary.totals.pending_cost));
  }

  function renderState(rows) {
    stateBody.replaceChildren();
    for (const item of rows) {
      const row = document.createElement("tr");
      row.append(td(item.name), td(intFmt.format(item.order_count), "num"), td(money(item.amount), "num"));
      stateBody.append(row);
    }
  }

  function renderSettlement(data) {
    settlementBody.replaceChildren();
    const rows = [
      ["Должно быть оплачено", data.due_order_count, data.due_amount],
      ["Оплачено", data.paid_order_count, data.paid_amount]
    ];
    for (const item of rows) {
      const row = document.createElement("tr");
      row.append(td(item[0]), td(intFmt.format(item[1]), "num"), td(money(item[2]), "num"));
      settlementBody.append(row);
    }
  }

  function renderFinancial(data) {
    financialBody.replaceChildren();
    financialCaption.textContent = `Валовая реализация: ${money(data.gross_realization)} · по заказам — вся история когорты; общие — исходные строки выбранного месяца без заказа. Сумма блока не обязана совпадать с выплатой когорты`;
    for (const item of data.rows) {
      const row = document.createElement("tr");
      const signClass = item.amount < 0 ? "negative" : (item.amount > 0 ? "positive" : "");
      row.append(
        td(item.name),
        td(money(item.amount), `num ${signClass}`),
        td(`${formatNumber(item.percent, 2)}%`, `num ${signClass}`)
      );
      financialBody.append(row);
    }
  }

  function renderChart(rows) {
    if (state.chart) {
      state.chart.destroy();
      state.chart = null;
    }
    if (!window.Chart) return;
    const period = monthPeriod(monthEl.value);
    const byDay = {};
    for (const row of rows) byDay[Number(row.event_date.slice(8, 10))] = row;
    const labels = [];
    const fields = ["order_amount", "realization", "ym_payment", "goods_return", "cancellation", "other_expenses"];
    const series = {};
    for (const field of fields) series[field] = [];
    for (let day = 1; day <= period.days; day += 1) {
      labels.push(String(day));
      const row = byDay[day] || {};
      for (const field of fields) series[field].push(Number(row[field]) || 0);
    }
    const primary = themeColor("--color-primary", "#2563eb");
    const success = themeColor("--color-success", "#16a34a");
    const warning = themeColor("--color-warning", "#d97706");
    const error = themeColor("--color-error", "#dc2626");
    const muted = themeColor("--color-info", "#7c3aed");
    state.chart = new window.Chart(chartCanvas.getContext("2d"), {
      type: "line",
      data: {
        labels,
        datasets: [
          { label: "Стоимость заказов", data: series.order_amount, borderColor: primary, backgroundColor: primary, tension: 0.2 },
          { label: "Реализации", data: series.realization, borderColor: muted, backgroundColor: muted, tension: 0.2 },
          { label: "От Яндекс Маркета", data: series.ym_payment, borderColor: success, backgroundColor: success, tension: 0.2 },
          { label: "Возвраты", data: series.goods_return, borderColor: error, backgroundColor: error, tension: 0.2 },
          { label: "Отказы", data: series.cancellation, borderColor: warning, backgroundColor: warning, tension: 0.2 },
          { label: "Прочие удержания YM", data: series.other_expenses, borderColor: "#64748b", backgroundColor: "#64748b", tension: 0.2 }
        ]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        interaction: { mode: "index", intersect: false },
        plugins: {
          legend: { position: "bottom" },
          tooltip: {
            callbacks: {
              afterBody: items => {
                const day = Number(labels[items[0].dataIndex]);
                const row = byDay[day] || {};
                return [
                  `Заказов: ${intFmt.format(row.order_count || 0)}`,
                  `Реализовано: ${intFmt.format(row.realized_count || 0)}`,
                  `Возвращено: ${intFmt.format(row.returned_count || 0)}`,
                  `Отказов: ${intFmt.format(row.cancelled_count || 0)}`,
                  `Оплачено реализованных: ${intFmt.format(row.settlement_paid_count || 0)}`,
                  `Есть расчёт YM: ${intFmt.format(row.paid_count || 0)}`
                ];
              }
            }
          }
        },
        scales: {
          x: { title: { display: true, text: "День месяца" } },
          y: { beginAtZero: true, ticks: { callback: value => formatNumber(value, 0) } }
        },
        onClick: (_event, elements) => {
          if (!elements.length) return;
          const day = Number(labels[elements[0].index]);
          state.selectedDay = `${monthEl.value}-${String(day).padStart(2, "0")}`;
          state.page = 1;
          clearDayEl.classList.remove("is-hidden");
          captionEl.textContent = `Заказы от ${humanDate(state.selectedDay)}`;
          loadOrders();
          activatePane("orders");
        }
      }
    });
  }

  function statusBadge(status) {
    const span = document.createElement("span");
    span.className = `order-status order-status--${String(status || "unknown").toLowerCase()}`;
    span.textContent = statusNames[status] || status || "Не найден";
    return span;
  }

  function renderSortIndicators() {
    for (const header of root.querySelectorAll("th[data-sort]")) {
      const base = header.dataset.label || header.textContent.replace(/\s[↑↓]$/, "");
      header.dataset.label = base;
      header.textContent = `${base}${header.dataset.sort === state.sortField ? (state.sortDirection === "asc" ? " ↑" : " ↓") : ""}`;
      header.classList.toggle("is-sorted", header.dataset.sort === state.sortField);
    }
  }

  function renderOrders(result) {
    state.lastOrders = result;
    state.totalRows = result.total_rows;
    ordersBody.replaceChildren();
    if (!result.rows.length) {
      const row = document.createElement("tr");
      const cell = td("Нет заказов за выбранный месяц", "empty-cell");
      cell.colSpan = 13;
      row.append(cell);
      ordersBody.append(row);
    } else {
      for (const item of result.rows) {
        const row = document.createElement("tr");
        let orderCell;
        if (item.order_ref) {
          orderCell = document.createElement("td");
          const link = document.createElement("a");
          link.className = "order-link mono";
          link.href = `?active=a013_ym_order_details_${encodeURIComponent(item.order_ref)}`;
          link.textContent = item.order_id || "—";
          link.addEventListener("click", event => {
            if (host.openTab) {
              event.preventDefault();
              host.openTab(`a013_ym_order_details_${item.order_ref}`, `Заказ ${item.order_id}`);
            }
          });
          orderCell.append(link);
        } else {
          orderCell = td(item.order_id || "—", "mono");
        }
        const statusCell = document.createElement("td");
        statusCell.append(statusBadge(item.order_status));
        row.append(
          td(humanDate(item.order_date)),
          orderCell,
          statusCell,
          td(money(item.order_amount), "num"),
          td(humanDate(item.realization_date)),
          td(money(item.realization_amount), "num"),
          td(item.return_amount ? money(-Math.abs(item.return_amount)) : money(0), `num ${item.return_amount ? "negative" : ""}`),
          td(humanDate(item.payment_date)),
          td(item.bank_order_id || "—", "mono"),
          td(item.cabinet_name || item.business_id),
          td(money(item.direct_net), `num ${item.direct_net < 0 ? "negative" : ""}`),
          td(money(item.allocated_shared), `num ${item.allocated_shared < 0 ? "negative" : ""}`),
          td(money(item.final_payment), `num final-money ${item.final_payment < 0 ? "negative" : ""}`)
        );
        ordersBody.append(row);
      }
    }
    const pages = Math.max(1, Math.ceil(state.totalRows / state.pageSize));
    const first = state.totalRows ? (state.page - 1) * state.pageSize + 1 : 0;
    const last = Math.min(state.page * state.pageSize, state.totalRows);
    pageInfoEl.textContent = `${first}–${last} из ${intFmt.format(state.totalRows)} · страница ${state.page} из ${pages}`;
    prevEl.disabled = state.loading || state.page <= 1;
    nextEl.disabled = state.loading || state.page >= pages;
    renderSortIndicators();
  }

  function renderCosts(rows) {
    costsBody.replaceChildren();
    if (!rows.length) {
      const row = document.createElement("tr");
      const cell = td("Общих затрат за выбранный месяц нет", "empty-cell");
      cell.colSpan = 8;
      row.append(cell);
      costsBody.append(row);
      return;
    }
    for (const item of rows) {
      const row = document.createElement("tr");
      const statusCell = document.createElement("td");
      const badge = document.createElement("span");
      badge.className = `badge badge--${item.settlement_state}`;
      badge.textContent = item.settlement_state === "settled"
        ? (item.amount >= 0 ? "Начислено" : "Удержано")
        : "Ожидается";
      statusCell.append(badge);
      if (item.payment_status) {
        const small = document.createElement("small");
        small.textContent = item.payment_status;
        statusCell.append(small);
      }
      row.append(
        td(humanDate(item.accrual_date)),
        td(item.act_date ? `${humanDate(item.act_date)}${item.act_id ? ` · ${item.act_id}` : ""}` : "—"),
        td(humanDate(item.payment_date)),
        td(item.bank_order_id || "—", "mono"),
        td(item.cabinet_name || item.business_id),
        td(`${item.transaction_source}${item.service_name ? ` · ${item.service_name}` : ""}`),
        statusCell,
        td(money(item.amount), `num ${item.amount < 0 ? "negative" : "positive"}`)
      );
      costsBody.append(row);
    }
  }

  async function loadOrders() {
    try {
      const result = await host.invoke("loadOrders", orderArgs(state.page, state.pageSize));
      renderOrders(result);
    } catch (error) {
      showError(error);
    }
  }

  async function exportCsv() {
    if (state.exporting) return;
    state.exporting = true;
    exportEl.disabled = true;
    exportEl.textContent = "Подготовка…";
    clearError();
    try {
      const allRows = [];
      let page = 1;
      let total = 0;
      do {
        const result = await host.invoke("loadOrders", orderArgs(page, 200));
        allRows.push(...result.rows);
        total = result.total_rows;
        page += 1;
        exportEl.textContent = `CSV ${Math.min(allRows.length, total)}/${total}`;
      } while (allRows.length < total);
      const headers = ["Дата заказа МСК", "Заказ", "Статус", "Цена заказа", "Дата реализации", "Сумма реализации", "Возврат", "Последняя оплата YM", "Поручения", "Кабинет", "Прямое нетто", "Общие расходы", "Получено от YM"];
      const lines = [headers.map(csvCell).join(";")];
      for (const item of allRows) {
        lines.push([
          item.order_date,
          item.order_id,
          statusNames[item.order_status] || item.order_status,
          item.order_amount,
          item.realization_date,
          item.realization_amount,
          item.return_amount ? -Math.abs(item.return_amount) : 0,
          item.payment_date,
          item.bank_order_id,
          item.cabinet_name || item.business_id,
          item.direct_net,
          item.allocated_shared,
          item.final_payment
        ].map(csvCell).join(";"));
      }
      const blob = new Blob(["\ufeff", lines.join("\r\n")], { type: "text/csv;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = `ym_cash_payments_${monthEl.value}.csv`;
      document.body.append(link);
      link.click();
      link.remove();
      URL.revokeObjectURL(url);
    } catch (error) {
      showError(error);
    } finally {
      state.exporting = false;
      exportEl.disabled = state.loading;
      exportEl.textContent = "Выгрузить CSV";
    }
  }

  async function loadAll() {
    clearError();
    state.page = 1;
    state.selectedDay = "";
    clearDayEl.classList.add("is-hidden");
    captionEl.textContent = "Все заказы выбранного месяца";
    setLoading(true);
    try {
      const [summary, orders, costs] = await Promise.all([
        host.invoke("loadSummary", args()),
        host.invoke("loadOrders", orderArgs(1, state.pageSize)),
        host.invoke("loadMonthlyCosts", args())
      ]);
      renderCards(summary);
      renderFreshness(summary.freshness);
      renderState(summary.order_state);
      renderSettlement(summary.settlement);
      renderFinancial(summary.financial_breakdown);
      renderChart(summary.daily_flow);
      renderOrders(orders);
      renderCosts(costs.rows);
    } catch (error) {
      showError(error);
    } finally {
      setLoading(false);
      if (state.lastOrders) renderOrders(state.lastOrders);
    }
  }

  refreshEl.addEventListener("click", loadAll);
  monthEl.addEventListener("change", loadAll);
  businessEl.addEventListener("change", loadAll);
  pageSizeEl.addEventListener("change", () => {
    state.pageSize = Number(pageSizeEl.value) || 50;
    state.page = 1;
    loadOrders();
  });
  prevEl.addEventListener("click", () => {
    if (state.page > 1) {
      state.page -= 1;
      loadOrders();
    }
  });
  nextEl.addEventListener("click", () => {
    if (state.page * state.pageSize < state.totalRows) {
      state.page += 1;
      loadOrders();
    }
  });
  clearDayEl.addEventListener("click", () => {
    state.selectedDay = "";
    state.page = 1;
    clearDayEl.classList.add("is-hidden");
    captionEl.textContent = "Все заказы выбранного месяца";
    loadOrders();
  });
  exportEl.addEventListener("click", exportCsv);

  for (const header of root.querySelectorAll("th[data-sort]")) {
    header.addEventListener("click", () => {
      const field = header.dataset.sort;
      if (state.sortField === field) {
        state.sortDirection = state.sortDirection === "asc" ? "desc" : "asc";
      } else {
        state.sortField = field;
        state.sortDirection = "asc";
      }
      state.page = 1;
      loadOrders();
    });
  }

  for (const tab of root.querySelectorAll(".tab")) {
    tab.addEventListener("click", () => {
      activatePane(tab.dataset.tab);
    });
  }

  try {
    const cabinets = await host.invoke("loadCabinets", {});
    if (cabinets.suggested_month) monthEl.value = cabinets.suggested_month;
    for (const cabinet of cabinets.rows) {
      const option = document.createElement("option");
      option.value = cabinet.business_id;
      option.textContent = cabinet.connections_count > 1
        ? `${cabinet.name} (${cabinet.business_id}, ${cabinet.connections_count} подключ.)`
        : `${cabinet.name} (${cabinet.business_id})`;
      businessEl.append(option);
    }
    await loadAll();
  } catch (error) {
    showError(error);
  }
}

export async function unmount() {
  // iframe lifecycle destroys Chart.js and event listeners with the document.
}

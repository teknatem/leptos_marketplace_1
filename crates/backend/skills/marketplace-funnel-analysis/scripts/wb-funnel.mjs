// WB-специфичный расчёт воронки из реальных полей p916 (см. references/wildberries-mapping.md).
// Отличия от generic calculate-funnel: показы = free+paid с флагом доступности (N/A ≠ 0),
// ветки отмен/возвратов, канальный сплит paid/free (free = total − paid, обрезка ≥0).

function finiteNumber(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

// Отношение с защитой от null/0 в знаменателе: возвращает null (N/A), а не 0.
function ratio(numerator, denominator) {
  if (numerator === null || denominator === null || denominator === 0) return null;
  return numerator / denominator;
}

// total − paid с обрезкой ≥0; N/A если платная сторона неизвестна.
function freeSide(total, paid) {
  if (total === null || paid === null) return null;
  return Math.max(0, total - paid);
}

export async function run(args, host) {
  const m = args.metrics || {};
  const warnings = [];

  const showFree = finiteNumber(m.show_free_count);
  const showPaid = finiteNumber(m.show_paid_count);
  // Общий total корректен только при наличии обеих компонент. Известную часть сохраняем
  // отдельно, но не выдаём платные показы за все показы при N/A органики.
  const knownShowSum = (showFree || 0) + (showPaid || 0);
  const showsAvailable = showFree !== null && showPaid !== null;
  const showTotal = showsAvailable ? knownShowSum : null;
  if (showFree === null && showPaid === null) {
    warnings.push({ code: "shows_unavailable", detail: "нет источника показов (нет «Джем»/рекламы a026)" });
  } else if (!showsAvailable) {
    warnings.push({
      code: "shows_partial",
      detail: "доступна только часть показов; общий total и конверсии от показа не рассчитываются"
    });
  }

  const opens = finiteNumber(m.open_count);
  const carts = finiteNumber(m.cart_count);
  const orders = finiteNumber(m.order_count);
  const cancels = finiteNumber(m.cancel_count);
  const buyouts = finiteNumber(m.buyout_count);
  const returns = finiteNumber(m.return_count);

  if (orders === null) {
    warnings.push({ code: "orders_missing", detail: "order_count отсутствует — воронка неполна" });
  }

  const stages = [
    { stage: "shows", value: showTotal, available: showsAvailable },
    { stage: "opens", value: opens, available: opens !== null },
    { stage: "carts", value: carts, available: carts !== null },
    { stage: "orders", value: orders, available: orders !== null },
    { stage: "buyouts", value: buyouts, available: buyouts !== null }
  ];

  // Немонотонность считаем только по доступным соседним этапам.
  const present = stages.filter((s) => s.value !== null);
  for (let i = 1; i < present.length; i += 1) {
    if (present[i].value > present[i - 1].value) {
      warnings.push({
        code: "non_monotonic_funnel",
        stage: present[i].stage,
        previous_stage: present[i - 1].stage
      });
    }
  }

  const conversions = {
    open_to_cart: ratio(carts, opens),
    cart_to_order: ratio(orders, carts),
    order_to_buyout: ratio(buyouts, orders),
    cancel_rate: ratio(cancels, orders),
    return_rate: ratio(returns, buyouts)
  };

  // Канальный сплit: платная сторона — из переданных paid_* (a026 / p913), free = total − paid.
  const paidOpen = finiteNumber(m.paid_open_count);
  const paidCart = finiteNumber(m.paid_cart_count);
  const paidOrder = finiteNumber(m.paid_order_count);
  const paidBuyout = finiteNumber(m.paid_buyout_count);
  const channelAvailable = showPaid !== null || paidOpen !== null || paidCart !== null
    || paidOrder !== null || paidBuyout !== null;
  const paidTrackComplete = showPaid !== null && paidOpen !== null && paidCart !== null
    && paidOrder !== null && paidBuyout !== null;
  if (channelAvailable && !paidTrackComplete) {
    warnings.push({
      code: "paid_track_incomplete",
      detail: "платная воронка неполна; нельзя заменять отсутствующие paid_* общими метриками"
    });
  }
  const channel = channelAvailable ? {
    // Показы: free — прямое поле show_free_count (органика), а не total−paid. null = N/A (не 0).
    shows: { total: showTotal, paid: showPaid, free: showFree },
    opens: { total: opens, paid: paidOpen, free: freeSide(opens, paidOpen) },
    carts: { total: carts, paid: paidCart, free: freeSide(carts, paidCart) },
    orders: { total: orders, paid: paidOrder, free: freeSide(orders, paidOrder) },
    buyouts: { total: buyouts, paid: paidBuyout, free: freeSide(buyouts, paidBuyout) },
    conversions: {
      show_to_open: ratio(paidOpen, showPaid),
      open_to_cart: ratio(paidCart, paidOpen),
      cart_to_order: ratio(paidOrder, paidCart),
      order_to_buyout: ratio(paidBuyout, paidOrder),
      show_to_buyout: ratio(paidBuyout, showPaid)
    },
    complete: paidTrackComplete
  } : null;
  if (!channelAvailable) {
    warnings.push({ code: "channel_unavailable", detail: "нет рекламных данных a026/p913 — платный трек N/A" });
  }

  // Наибольший спад среди доступных пар — кандидат для диагностики.
  let largestDrop = null;
  for (let i = 1; i < present.length; i += 1) {
    const conv = ratio(present[i].value, present[i - 1].value);
    if (conv === null) continue;
    const drop = 1 - conv;
    if (largestDrop === null || drop > largestDrop.drop_off) {
      largestDrop = { from: present[i - 1].stage, to: present[i].stage, drop_off: drop };
    }
  }

  host.log.info("wb funnel calculated", args.label || "", warnings.length);

  return {
    marketplace: "wildberries",
    label: args.label || null,
    shows: {
      free: showFree,
      paid: showPaid,
      total: showTotal,
      known_components_sum: knownShowSum,
      available: showsAvailable
    },
    stages,
    conversions,
    cancels,
    returns,
    order_sum: finiteNumber(m.order_sum),
    channel,
    largest_drop: largestDrop,
    warnings
  };
}

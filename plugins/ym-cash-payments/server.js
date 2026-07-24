function text(value) {
  return value == null ? "" : String(value);
}

function num(value) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function requirePeriod(args) {
  if (!args || !args.dateFrom || !args.dateTo) {
    throw new Error("Укажите начало и конец периода");
  }
  if (String(args.dateFrom) > String(args.dateTo)) {
    throw new Error("Начало периода не может быть позже окончания");
  }
}

function cabinet(args) {
  return args && args.businessId ? String(args.businessId) : "";
}

function normalizeOrder(row) {
  return {
    business_id: text(row.business_id),
    cabinet_name: text(row.cabinet_name),
    order_date: text(row.order_date),
    payment_date: text(row.payment_date),
    first_payment_date: text(row.first_payment_date),
    bank_order_id: text(row.bank_order_id),
    payment_orders: num(row.payment_orders),
    order_id: text(row.order_id),
    order_ref: text(row.order_ref),
    order_status: text(row.order_status),
    order_amount: num(row.order_amount),
    realization_date: text(row.realization_date),
    realization_amount: num(row.realization_amount),
    return_amount: num(row.return_amount),
    buyer_payment: num(row.buyer_payment),
    direct_net: num(row.direct_net),
    allocated_shared: num(row.allocated_shared),
    final_payment: num(row.final_payment),
    unallocated: num(row.unallocated),
    total_rows: num(row.total_rows)
  };
}

export async function loadCabinets(_args, host) {
  const [rows, freshnessRows] = await Promise.all([
    host.db.queryResource("cabinets", []),
    host.db.queryResource("freshness", ["", ""])
  ]);
  const freshness = freshnessRows.length ? freshnessRows[0] : {};
  const lastDate = text(freshness.last_transaction_date)
    || text(freshness.last_bank_order_date);
  return {
    rows: rows.map((row) => ({
      business_id: text(row.business_id),
      name: text(row.name),
      shops: text(row.shops),
      connections_count: num(row.connections_count)
    })),
    suggested_month: lastDate.length >= 7 ? lastDate.slice(0, 7) : ""
  };
}

export async function loadSummary(args, host) {
  requirePeriod(args);
  const businessId = cabinet(args);
  const [
    pendingRows,
    freshnessRows,
    stateRows,
    dueRows,
    flowRows,
    monthlyCostRows
  ] = await Promise.all([
    host.db.queryResource("pendingSummary", [
      args.dateFrom,
      args.dateTo,
      businessId,
      businessId
    ]),
    host.db.queryResource("freshness", [businessId, businessId]),
    host.db.queryResource("orderStateSummary", [
      args.dateFrom,
      args.dateTo,
      businessId,
      businessId
    ]),
    host.db.queryResource("settlementDue", [
      args.dateFrom,
      args.dateTo,
      businessId,
      businessId
    ]),
    host.db.queryResource("dailyFlow", [
      args.dateFrom,
      args.dateTo,
      businessId,
      businessId,
      businessId,
      businessId
    ]),
    host.db.queryResource("monthlyCostSummary", [
      args.dateFrom,
      args.dateTo,
      businessId,
      businessId
    ])
  ]);

  const dailyFlow = flowRows.map((row) => ({
    event_date: text(row.event_date),
    order_count: num(row.order_count),
    order_amount: num(row.order_amount),
    realized_count: num(row.realized_count),
    realization: num(row.realization),
    returned_count: num(row.returned_count),
    goods_return: num(row.goods_return),
    cancelled_count: num(row.cancelled_count),
    cancellation: num(row.cancellation),
    paid_count: num(row.paid_count),
    settlement_paid_count: num(row.settlement_paid_count),
    settlement_paid_amount: num(row.settlement_paid_amount),
    ym_payment: num(row.ym_payment),
    order_accruals: num(row.order_accruals),
    order_withholdings: num(row.order_withholdings),
    common_accruals: num(row.common_accruals),
    common_withholdings: num(row.common_withholdings),
    other_expenses: num(row.other_expenses)
  }));
  const totals = dailyFlow.reduce(
    (acc, row) => {
      acc.bank_sum += row.ym_payment;
      acc.order_count += row.settlement_paid_count;
      acc.allocated_shared += row.other_expenses;
      acc.final_payment += row.ym_payment;
      acc.settlement_paid_amount += row.settlement_paid_amount;
      acc.gross_realization += row.realization;
      acc.order_accruals += row.order_accruals;
      acc.order_withholdings += row.order_withholdings;
      acc.common_accruals += row.common_accruals;
      acc.common_withholdings += row.common_withholdings;
      return acc;
    },
    {
      bank_sum: 0,
      order_count: 0,
      allocated_shared: 0,
      final_payment: 0,
      settlement_paid_amount: 0,
      gross_realization: 0,
      order_accruals: 0,
      order_withholdings: 0,
      common_accruals: 0,
      common_withholdings: 0,
      unallocated: 0,
      pending_cost: 0
    }
  );
  totals.pending_cost = pendingRows.length ? Math.abs(num(pendingRows[0].pending_cost)) : 0;

  const orderState = stateRows.map((row) => ({
    key: text(row.state_key),
    name: text(row.state_name),
    order_count: num(row.orders_count),
    amount: num(row.amount)
  }));
  const due = dueRows.length ? dueRows[0] : {};
  const settlement = {
    due_order_count: num(due.due_order_count),
    due_amount: num(due.due_amount),
    paid_order_count: totals.order_count,
    paid_amount: totals.settlement_paid_amount
  };
  const monthlyCostRow = monthlyCostRows.length ? monthlyCostRows[0] : {};
  const percentOfGross = (amount) => totals.gross_realization
    ? amount * 100 / totals.gross_realization
    : 0;
  const financialBreakdown = {
    gross_realization: totals.gross_realization,
    rows: [
      {
        key: "order_accruals",
        name: "Начисления по заказам",
        amount: totals.order_accruals,
        percent: percentOfGross(totals.order_accruals)
      },
      {
        key: "order_withholdings",
        name: "Удержания по заказам",
        amount: totals.order_withholdings,
        percent: percentOfGross(totals.order_withholdings)
      },
      {
        key: "common_accruals",
        name: "Общие начисления",
        amount: num(monthlyCostRow.common_accruals),
        percent: percentOfGross(num(monthlyCostRow.common_accruals))
      },
      {
        key: "common_withholdings",
        name: "Общие удержания",
        amount: num(monthlyCostRow.common_withholdings),
        percent: percentOfGross(num(monthlyCostRow.common_withholdings))
      }
    ]
  };
  const monthlyCost = {
    common_accruals: num(monthlyCostRow.common_accruals),
    common_withholdings: num(monthlyCostRow.common_withholdings),
    total_cost: num(monthlyCostRow.total_cost),
    settled_cost: num(monthlyCostRow.settled_cost),
    pending_cost: num(monthlyCostRow.pending_cost)
  };

  const freshness = freshnessRows.length
    ? {
        loaded_at_utc: text(freshnessRows[0].loaded_at_utc),
        last_transaction_date: text(freshnessRows[0].last_transaction_date),
        last_bank_order_date: text(freshnessRows[0].last_bank_order_date)
      }
    : {
        loaded_at_utc: "",
        last_transaction_date: "",
        last_bank_order_date: ""
      };

  host.log.info(
    "YM cash summary",
    args.dateFrom,
    args.dateTo,
    businessId || "(all cabinets)",
    "bank:",
    totals.bank_sum
  );
  return {
    daily: [],
    daily_flow: dailyFlow,
    totals,
    order_state: orderState,
    settlement,
    financial_breakdown: financialBreakdown,
    monthly_cost: monthlyCost,
    freshness
  };
}

export async function loadOrders(args, host) {
  requirePeriod(args);
  const businessId = cabinet(args);
  const paymentDay = args.paymentDay ? String(args.paymentDay) : "";
  const pageSize = Math.max(20, Math.min(200, Math.trunc(num(args.pageSize) || 50)));
  const page = Math.max(1, Math.trunc(num(args.page) || 1));
  const offset = (page - 1) * pageSize;
  const allowedSortFields = [
    "order_date",
    "payment_date",
    "bank_order_id",
    "order_id",
    "order_status",
    "order_amount",
    "realization_date",
    "cabinet_name",
    "buyer_payment",
    "direct_net",
    "allocated_shared",
    "final_payment"
  ];
  const requestedSort = text(args.sortField);
  const sortField = allowedSortFields.indexOf(requestedSort) >= 0
    ? requestedSort
    : "order_date";
  const sortDirection = text(args.sortDirection) === "asc" ? "asc" : "desc";
  const rows = await host.db.queryResource("orders", [
    args.dateFrom,
    args.dateTo,
    businessId,
    businessId,
    businessId,
    businessId,
    paymentDay,
    paymentDay,
    sortDirection,
    sortField,
    sortDirection,
    sortField,
    pageSize,
    offset
  ]);
  const normalized = rows.map(normalizeOrder);
  return {
    rows: normalized,
    page,
    page_size: pageSize,
    total_rows: normalized.length ? normalized[0].total_rows : 0,
    payment_day: paymentDay,
    sort_field: sortField,
    sort_direction: sortDirection
  };
}

export async function loadMonthlyCosts(args, host) {
  requirePeriod(args);
  const businessId = cabinet(args);
  const rows = await host.db.queryResource("monthlyCosts", [
    args.dateFrom,
    args.dateTo,
    businessId,
    businessId,
    args.dateFrom,
    args.dateTo,
    businessId,
    businessId
  ]);
  return {
    rows: rows.map((row) => ({
      business_id: text(row.business_id),
      cabinet_name: text(row.cabinet_name),
      accrual_date: text(row.accrual_date),
      act_date: text(row.act_date),
      act_id: text(row.act_id),
      payment_date: text(row.payment_date),
      bank_order_id: text(row.bank_order_id),
      transaction_source: text(row.transaction_source),
      service_name: text(row.service_name),
      payment_status: text(row.payment_status),
      settlement_state: text(row.settlement_state),
      amount: num(row.amount),
      rows_count: num(row.rows_count)
    }))
  };
}

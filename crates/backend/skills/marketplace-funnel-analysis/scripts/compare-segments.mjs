const STAGES = ["impressions", "clicks", "product_views", "cart_adds", "orders", "deliveries"];

function number(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function segmentResult(item) {
  const available = STAGES
    .map((stage) => ({ stage, value: number(item.metrics[stage]) }))
    .filter((metric) => metric.value !== null);
  const warnings = [];
  for (let index = 1; index < available.length; index += 1) {
    if (available[index].value > available[index - 1].value) {
      warnings.push({ code: "non_monotonic_funnel", stage: available[index].stage });
    }
  }
  const start = available.length > 0 ? available[0].value : null;
  const orders = number(item.metrics.orders);
  return {
    segment: item.segment,
    orders,
    revenue: number(item.metrics.revenue),
    start_stage: available.length > 0 ? available[0].stage : null,
    conversion_to_order: start === null || start === 0 || orders === null ? null : orders / start,
    warnings
  };
}

export async function run(args, host) {
  const segments = args.segments.map(segmentResult);
  segments.sort((left, right) => (right.orders || 0) - (left.orders || 0));
  const validForRanking = segments.filter((segment) => segment.warnings.length === 0);
  host.log.info("segments compared", segments.length);
  return {
    marketplace: args.marketplace,
    segments,
    ranking_by_orders: validForRanking.map((segment) => segment.segment),
    excluded_from_ranking: segments
      .filter((segment) => segment.warnings.length > 0)
      .map((segment) => segment.segment)
  };
}

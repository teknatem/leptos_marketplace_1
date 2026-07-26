const STAGES = ["impressions", "clicks", "product_views", "cart_adds", "orders", "deliveries"];

function finiteNumber(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function calculate(metrics) {
  const available = STAGES
    .map((stage) => ({ stage, value: finiteNumber(metrics[stage]) }))
    .filter((item) => item.value !== null);
  const first = available.length > 0 ? available[0].value : null;
  return available.map((item, index) => {
    const previous = index > 0 ? available[index - 1].value : null;
    const fromPrevious = previous === null || previous === 0 ? null : item.value / previous;
    return {
      stage: item.stage,
      value: item.value,
      conversion_from_previous: fromPrevious,
      conversion_from_start: first === null || first === 0 ? null : item.value / first,
      drop_off: fromPrevious === null ? null : 1 - fromPrevious
    };
  });
}

export async function run(args, host) {
  const stages = calculate(args.metrics);
  const warnings = [];
  for (let index = 1; index < stages.length; index += 1) {
    if (stages[index].value > stages[index - 1].value) {
      warnings.push({
        code: "non_monotonic_funnel",
        stage: stages[index].stage,
        previous_stage: stages[index - 1].stage
      });
    }
  }
  const largestDrop = stages
    .filter((item) => item.drop_off !== null)
    .reduce((best, item) => best === null || item.drop_off > best.drop_off ? item : best, null);
  host.log.info("funnel calculated", args.marketplace, args.label || "");
  return {
    marketplace: args.marketplace,
    label: args.label || null,
    stages,
    revenue: finiteNumber(args.metrics.revenue),
    largest_drop: largestDrop === null ? null : {
      stage: largestDrop.stage,
      drop_off: largestDrop.drop_off
    },
    warnings
  };
}

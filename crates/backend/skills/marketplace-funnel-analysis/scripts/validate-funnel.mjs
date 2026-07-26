const STAGES = ["impressions", "clicks", "product_views", "cart_adds", "orders", "deliveries"];

export async function run(args, host) {
  const errors = [];
  const warnings = [];
  const present = [];
  for (const stage of STAGES) {
    const value = args.metrics[stage];
    if (value === undefined || value === null) continue;
    if (typeof value !== "number" || !Number.isFinite(value)) {
      errors.push({ code: "not_finite_number", field: stage });
      continue;
    }
    if (value < 0) errors.push({ code: "negative_value", field: stage, value });
    present.push({ stage, value });
  }
  if (typeof args.metrics.orders !== "number" || !Number.isFinite(args.metrics.orders)) {
    errors.push({ code: "orders_missing_or_invalid", field: "orders" });
  }
  for (let index = 1; index < present.length; index += 1) {
    if (present[index].value > present[index - 1].value) {
      warnings.push({
        code: "non_monotonic_funnel",
        stage: present[index].stage,
        previous_stage: present[index - 1].stage,
        value: present[index].value,
        previous_value: present[index - 1].value
      });
    }
  }
  const missing_stages = STAGES.filter((stage) => args.metrics[stage] === undefined || args.metrics[stage] === null);
  host.log.info("funnel validation", errors.length, warnings.length);
  return {
    valid: errors.length === 0,
    marketplace: args.marketplace,
    errors,
    warnings,
    missing_stages,
    available_stages: present.map((item) => item.stage)
  };
}

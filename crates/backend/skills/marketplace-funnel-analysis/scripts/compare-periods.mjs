const METRICS = ["impressions", "clicks", "product_views", "cart_adds", "orders", "deliveries", "revenue"];

function number(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function delta(current, previous) {
  if (current === null || previous === null) return { absolute: null, relative: null };
  return {
    absolute: current - previous,
    relative: previous === 0 ? null : (current - previous) / previous
  };
}

function conversion(metrics, from, to) {
  const left = number(metrics[from]);
  const right = number(metrics[to]);
  return left === null || right === null || left === 0 ? null : right / left;
}

export async function run(args, host) {
  const metrics = {};
  for (const name of METRICS) {
    const current = number(args.current.metrics[name]);
    const previous = number(args.previous.metrics[name]);
    metrics[name] = { current, previous, ...delta(current, previous) };
  }
  const conversionPairs = [
    ["impressions", "clicks"],
    ["clicks", "product_views"],
    ["product_views", "cart_adds"],
    ["cart_adds", "orders"],
    ["orders", "deliveries"]
  ];
  const conversions = conversionPairs.map(([from, to]) => {
    const current = conversion(args.current.metrics, from, to);
    const previous = conversion(args.previous.metrics, from, to);
    return { from, to, current, previous, ...delta(current, previous) };
  });
  host.log.info("periods compared", args.previous.label, args.current.label);
  return {
    marketplace: args.marketplace,
    current_label: args.current.label,
    previous_label: args.previous.label,
    metrics,
    conversions,
    warnings: args.current.label === args.previous.label
      ? [{ code: "period_labels_are_equal" }]
      : []
  };
}

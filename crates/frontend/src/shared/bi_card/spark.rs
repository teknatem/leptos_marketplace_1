//! Sparkline SVG path generator for BI indicator cards.

/// Generate SVG `d` attribute strings for a sparkline.
///
/// Returns `(line_d, fill_d)` where:
/// - `line_d` is a polyline path (M … L … L …)
/// - `fill_d` is the same with the bottom closed (for a filled area)
///
/// The output is normalised to fit in a `100 x 30` viewBox.
pub fn points_to_svg_path(points: &[f64]) -> (String, String) {
    if points.is_empty() {
        let line = "M0 15 L100 15".to_string();
        let fill = "M0 15 L100 15 L100 30 L0 30 Z".to_string();
        return (line, fill);
    }

    let min = points.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = points.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = if (max - min).abs() < 1e-9 {
        1.0
    } else {
        max - min
    };

    let n = points.len();
    let coords: Vec<(f64, f64)> = points
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = if n == 1 {
                50.0
            } else {
                i as f64 / (n - 1) as f64 * 100.0
            };
            // y: 0 = top, so invert; leave 2px margin top/bottom within 30px
            let y = 28.0 - (v - min) / range * 24.0;
            (x, y)
        })
        .collect();

    let mut line = String::new();
    for (i, (x, y)) in coords.iter().enumerate() {
        if i == 0 {
            line.push_str(&format!("M{:.1} {:.1}", x, y));
        } else {
            line.push_str(&format!(" L{:.1} {:.1}", x, y));
        }
    }

    let fill = format!(
        "{} L{:.1} 30 L{:.1} 30 Z",
        line,
        coords.last().map(|p| p.0).unwrap_or(100.0),
        coords.first().map(|p| p.0).unwrap_or(0.0),
    );

    (line, fill)
}

/// Default demo sparkline data (8 points, upward trend with noise).
pub fn demo_spark_points() -> Vec<f64> {
    vec![23.0, 22.0, 18.0, 19.0, 14.0, 12.0, 9.0, 7.0]
}

# D400 Monthly Summary Dashboard

Standalone dashboard component for displaying monthly marketplace summary data. Can be embedded in an iframe and initialized with a JavaScript function.

## Files

- `dashboard.html` - Main HTML structure
- `dashboard.css` - Scoped styles (all styles prefixed with `#bolt-root`)
- `dashboard.js` - JavaScript logic with global `render()` function
- `demo.html` - Demo page with sample data

## Usage

### Option 1: Basic Usage (No Period Selector)

```html
<!DOCTYPE html>
<html>
  <head>
    <link rel="stylesheet" href="dashboard.css" />
  </head>
  <body>
    <div id="bolt-root"></div>
    <script src="dashboard.js"></script>
    <script>
      const data = {
        "period": "2025-12",
        "marketplaces": ["WB", "OZON", "YM"],
        "rows": [...]
      };

      const container = document.getElementById('bolt-root');
      window.render(container, data);
    </script>
  </body>
</html>
```

### Option 2: With Period Selector

```html
<!DOCTYPE html>
<html>
  <head>
    <link rel="stylesheet" href="dashboard.css" />
  </head>
  <body>
    <div id="bolt-root"></div>
    <script src="dashboard.js"></script>
    <script>
      let currentPeriod = "2025-12";
      const dataByPeriod = {
        "2025-11": { period: "2025-11", marketplaces: [...], rows: [...] },
        "2025-12": { period: "2025-12", marketplaces: [...], rows: [...] }
      };

      function renderDashboard() {
        const container = document.getElementById('bolt-root');
        window.render(container, dataByPeriod[currentPeriod], {
          availablePeriods: ["2025-11", "2025-12"],
          onPeriodChange: function(newPeriod) {
            currentPeriod = newPeriod;
            renderDashboard();
          }
        });
      }

      renderDashboard();
    </script>
  </body>
</html>
```

### Option 3: iframe Integration

```html
<!-- Parent page -->
<iframe
  id="dashboard-frame"
  src="dashboard.html"
  width="100%"
  height="600"
></iframe>

<script>
  const iframe = document.getElementById('dashboard-frame');
  iframe.onload = function() {
    const iframeWindow = iframe.contentWindow;
    const container = iframeWindow.document.getElementById('bolt-root');

    const data = { period: "2025-12", marketplaces: [...], rows: [...] };
    iframeWindow.render(container, data);
  };
</script>
```

## API

### `render(container, data, options)`

Renders the dashboard into the specified container.

**Parameters:**

- `container` (HTMLElement) - DOM element to render into (must have id="bolt-root")
- `data` (Object) - Dashboard data object
- `options` (Object, optional) - Configuration options
  - `availablePeriods` (Array<string>) - List of periods for the dropdown selector
  - `onPeriodChange` (Function) - Callback when user selects a different period

**Data Format:**

```javascript
{
  "period": "2025-12",           // Report period
  "marketplaces": ["WB", "OZON", "YM"],  // Marketplace columns
  "rows": [
    {
      "indicator_id": "revenue",     // Unique indicator ID
      "indicator_name": "Выручка",   // Display name
      "group_name": null,            // Organization name (null for totals)
      "level": 0,                    // 0 = total, 1 = organization detail
      "values": {
        "WB": 1250000.0,
        "OZON": 780000.0,
        "YM": 210000.0,
        "total": 2240000.0
      },
      "drilldown_filter": { ... }    // Optional filter data
    }
  ]
}
```

## Features

- **No Dependencies**: Pure vanilla JavaScript, no external libraries
- **Scoped Styles**: All CSS scoped to `#bolt-root`, won't affect parent page
- **Number Formatting**: Automatic thousand separators and 2 decimal places
- **Hierarchical Display**: Shows totals (level 0) and organization details (level 1)
- **Percentage Columns**: Separate columns for percentages (8 value columns total: WB, WB%, OZON, OZON%, YM, YM%, Итого, Итого%)
- **100% Base**: All percentages calculated from total revenue (top-right cell in revenue section)
- **Column Sorting**: Click any column header to sort by that column (ascending/descending)
- **Period Selector**: Optional dropdown to switch between different reporting periods
- **Grouped Indicators**: Automatically groups rows by indicator_id
- **Visual Distinction**: Returns displayed in red, totals highlighted in blue, percentages in muted colors
- **Responsive**: Horizontal scroll for narrow viewports
- **Interactive**: Smooth transitions, hover states, and visual feedback
- **Fallbacks**: Handles missing data gracefully

## Style Guarantees

All styles are strictly scoped to `#bolt-root`:

- No global selectors (html, body, \*, :root)
- No reset/normalize styles
- No modifications to parent page styles
- Safe for iframe embedding

## Testing

Open `demo.html` in a browser to see the dashboard with sample data.

## Browser Support

Works in all modern browsers (Chrome, Firefox, Safari, Edge).

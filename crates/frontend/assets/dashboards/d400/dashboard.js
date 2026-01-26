(function() {
    'use strict';
  
    // State for sorting
    let currentSortColumn = null;
    let currentSortDirection = 'desc';
  
    /**
     * Format number with thousand separators and 2 decimal places
     * @param {number} value - Number to format
     * @returns {string} Formatted number
     */
    function formatNumber(value) {
      if (value == null || isNaN(value)) {
        return '0.00';
      }
      return new Intl.NumberFormat('ru-RU', {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2
      }).format(value);
    }
  
    /**
     * Format percentage
     * @param {number} value - Percentage value
     * @returns {string} Formatted percentage
     */
    function formatPercentage(value) {
      if (value == null || isNaN(value)) {
        return '0%';
      }
      return value.toFixed(1) + '%';
    }
  
    /**
     * Get value from row values object, fallback to 0
     * @param {object} values - Values object
     * @param {string} key - Key to retrieve
     * @returns {number} Value or 0
     */
    function getValue(values, key) {
      if (!values || values[key] == null) {
        return 0;
      }
      return values[key];
    }
  
    /**
     * Calculate percentage relative to total
     * @param {number} value - Value
     * @param {number} total - Total value
     * @returns {number} Percentage
     */
    function calculatePercentage(value, total) {
      if (!total || total === 0) {
        return 0;
      }
      return (value / total) * 100;
    }
  
    /**
     * Create empty state element
     * @returns {HTMLElement} Empty state element
     */
    function createEmptyState() {
      const emptyState = document.createElement('div');
      emptyState.className = 'empty-state';
      emptyState.innerHTML = `
        <div class="empty-state-icon">📊</div>
        <p class="empty-state-text">Нет данных для отображения</p>
      `;
      return emptyState;
    }
  
    /**
     * Create table header with sorting
     * @param {Array<string>} marketplaces - List of marketplace codes
     * @param {Function} onSort - Callback for sorting
     * @returns {HTMLElement} Table header element
     */
    function createTableHeader(marketplaces, onSort) {
      const thead = document.createElement('thead');
      const tr = document.createElement('tr');
  
      const thIndicator = document.createElement('th');
      thIndicator.textContent = 'Показатель';
      tr.appendChild(thIndicator);
  
      // Create value + percentage columns for each marketplace
      marketplaces.forEach(function(marketplace) {
        // Value column
        const thValue = document.createElement('th');
        thValue.className = 'numeric sortable';
        thValue.dataset.column = marketplace;
  
        const contentValue = document.createElement('span');
        contentValue.textContent = marketplace;
        thValue.appendChild(contentValue);
  
        const arrowValue = document.createElement('span');
        arrowValue.className = 'sort-arrow';
        thValue.appendChild(arrowValue);
  
        thValue.addEventListener('click', function() {
          onSort(marketplace);
        });
  
        tr.appendChild(thValue);
  
        // Percentage column
        const thPercent = document.createElement('th');
        thPercent.className = 'numeric percent-column';
        thPercent.textContent = '%';
        tr.appendChild(thPercent);
      });
  
      // Total value column
      const thTotal = document.createElement('th');
      thTotal.className = 'numeric sortable';
      thTotal.dataset.column = 'total';
  
      const contentTotal = document.createElement('span');
      contentTotal.textContent = 'Итого';
      thTotal.appendChild(contentTotal);
  
      const arrowTotal = document.createElement('span');
      arrowTotal.className = 'sort-arrow';
      thTotal.appendChild(arrowTotal);
  
      thTotal.addEventListener('click', function() {
        onSort('total');
      });
  
      tr.appendChild(thTotal);
  
      // Total percentage column
      const thTotalPercent = document.createElement('th');
      thTotalPercent.className = 'numeric percent-column';
      thTotalPercent.textContent = '%';
      tr.appendChild(thTotalPercent);
  
      thead.appendChild(tr);
      return thead;
    }
  
    /**
     * Group rows by indicator_id
     * @param {Array<object>} rows - Array of row objects
     * @returns {object} Grouped rows by indicator_id
     */
    function groupByIndicator(rows) {
      const groups = {};
      rows.forEach(function(row) {
        const indicatorId = row.indicator_id || 'unknown';
        if (!groups[indicatorId]) {
          groups[indicatorId] = [];
        }
        groups[indicatorId].push(row);
      });
      return groups;
    }
  
    /**
     * Create table body
     * @param {Array<string>} marketplaces - List of marketplace codes
     * @param {Array<object>} rows - Array of row objects
     * @param {string} sortColumn - Column to sort by
     * @param {string} sortDirection - Sort direction (asc/desc)
     * @returns {HTMLElement} Table body element
     */
    function createTableBody(marketplaces, rows, sortColumn, sortDirection) {
      const tbody = document.createElement('tbody');
  
      if (!rows || rows.length === 0) {
        return tbody;
      }
  
      const grouped = groupByIndicator(rows);
      const indicatorIds = Object.keys(grouped);
  
      // Find the base 100% value: total revenue from the first "revenue" indicator at level 0
      let baseTotal = 0;
      const revenueRows = grouped['revenue'] || [];
      const revenueLevel0 = revenueRows.find(function(r) { return r.level === 0; });
      if (revenueLevel0) {
        baseTotal = getValue(revenueLevel0.values, 'total');
      }
  
      indicatorIds.forEach(function(indicatorId, index) {
        const indicatorRows = grouped[indicatorId];
  
        // Sort by level (0 first, then 1), then by sort column if specified
        indicatorRows.sort(function(a, b) {
          const levelDiff = (a.level || 0) - (b.level || 0);
          if (levelDiff !== 0) {
            return levelDiff;
          }
  
          // Sort level-1 rows by selected column
          if (sortColumn && a.level === 1 && b.level === 1) {
            const aVal = getValue(a.values, sortColumn);
            const bVal = getValue(b.values, sortColumn);
            return sortDirection === 'desc' ? bVal - aVal : aVal - bVal;
          }
  
          return 0;
        });
  
        indicatorRows.forEach(function(row) {
          const tr = document.createElement('tr');
          tr.className = 'level-' + (row.level || 0);
          tr.dataset.indicatorId = indicatorId;

          if (indicatorId === 'returns') {
            tr.classList.add('indicator-returns');
          }
          if (indicatorId === 'cost') {
            tr.classList.add('indicator-cost');
          }
          if (indicatorId === 'result') {
            tr.classList.add('indicator-result');
          }

          const tdLabel = document.createElement('td');
          tdLabel.className = 'label';
  
          if (row.level === 0 || !row.group_name) {
            tdLabel.textContent = row.indicator_name || indicatorId;
          } else {
            tdLabel.textContent = row.group_name;
          }
  
          tr.appendChild(tdLabel);
  
          // Add value + percentage columns for each marketplace
          marketplaces.forEach(function(marketplace) {
            // Value column
            const tdValue = document.createElement('td');
            tdValue.className = 'numeric';
            const value = getValue(row.values, marketplace);
            tdValue.textContent = formatNumber(value);
            tr.appendChild(tdValue);
  
            // Percentage column
            const tdPercent = document.createElement('td');
            tdPercent.className = 'numeric percent-column';
            if (baseTotal > 0) {
              const percentage = calculatePercentage(value, baseTotal);
              tdPercent.textContent = formatPercentage(percentage);
            } else {
              tdPercent.textContent = '0%';
            }
            tr.appendChild(tdPercent);
          });
  
          // Total value column
          const tdTotal = document.createElement('td');
          tdTotal.className = 'numeric';
          const totalValue = getValue(row.values, 'total');
          tdTotal.textContent = formatNumber(totalValue);
          tr.appendChild(tdTotal);
  
          // Total percentage column
          const tdTotalPercent = document.createElement('td');
          tdTotalPercent.className = 'numeric percent-column';
          if (baseTotal > 0) {
            const percentage = calculatePercentage(totalValue, baseTotal);
            tdTotalPercent.textContent = formatPercentage(percentage);
          } else {
            tdTotalPercent.textContent = '0%';
          }
          tr.appendChild(tdTotalPercent);
  
          tbody.appendChild(tr);
        });
  
        // Add separator between indicators (except after last one)
        if (index < indicatorIds.length - 1) {
          const separator = document.createElement('tr');
          separator.className = 'indicator-separator';
          const td = document.createElement('td');
          // Now we have: 1 label + (marketplaces.length * 2) + 2 total columns
          td.colSpan = 1 + (marketplaces.length * 2) + 2;
          separator.appendChild(td);
          tbody.appendChild(separator);
        }
      });
  
      return tbody;
    }
  
    /**
     * Create dashboard header with period selector
     * @param {string} period - Current period string (e.g., "2025-12")
     * @param {Array<string>} availablePeriods - List of available periods
     * @param {Function} onPeriodChange - Callback for period change
     * @returns {HTMLElement} Header element
     */
    function createHeader(period, availablePeriods, onPeriodChange) {
      const header = document.createElement('div');
      header.className = 'dashboard-header';
  
      const topRow = document.createElement('div');
      topRow.className = 'header-top';
  
      const title = document.createElement('h1');
      title.className = 'dashboard-title';
      title.textContent = 'Сводка по маркетплейсам';
      topRow.appendChild(title);
  
      // Period selector
      if (availablePeriods && availablePeriods.length > 0 && onPeriodChange) {
        const selectorWrapper = document.createElement('div');
        selectorWrapper.className = 'period-selector-wrapper';
  
        const label = document.createElement('label');
        label.className = 'period-label';
        label.textContent = 'Период:';
        label.setAttribute('for', 'period-select');
  
        const select = document.createElement('select');
        select.id = 'period-select';
        select.className = 'period-selector';
  
        availablePeriods.forEach(function(p) {
          const option = document.createElement('option');
          option.value = p;
          option.textContent = p;
          if (p === period) {
            option.selected = true;
          }
          select.appendChild(option);
        });
  
        select.addEventListener('change', function() {
          onPeriodChange(select.value);
        });
  
        selectorWrapper.appendChild(label);
        selectorWrapper.appendChild(select);
        topRow.appendChild(selectorWrapper);
      } else {
        const periodEl = document.createElement('p');
        periodEl.className = 'dashboard-period';
        periodEl.textContent = 'Отчет за ' + (period || 'неизвестный период');
        topRow.appendChild(periodEl);
      }
  
      header.appendChild(topRow);
  
      return header;
    }
  
    /**
     * Update sort indicators in table header
     * @param {HTMLElement} table - Table element
     * @param {string} column - Column name
     * @param {string} direction - Sort direction
     */
    function updateSortIndicators(table, column, direction) {
      const headers = table.querySelectorAll('th.sortable');
      headers.forEach(function(th) {
        const arrow = th.querySelector('.sort-arrow');
        if (th.dataset.column === column) {
          th.classList.add('sorted');
          arrow.textContent = direction === 'desc' ? ' ▼' : ' ▲';
        } else {
          th.classList.remove('sorted');
          arrow.textContent = '';
        }
      });
    }
  
    /**
     * Main render function
     * @param {HTMLElement} container - Container element to render into
     * @param {object} data - Data object with period, marketplaces, and rows
     * @param {object} options - Optional configuration object
     * @param {Array<string>} options.availablePeriods - List of available periods for selector
     * @param {Function} options.onPeriodChange - Callback when period changes
     */
    function render(container, data, options) {
      if (!container) {
        console.error('Dashboard: container is required');
        return;
      }
  
      if (!data) {
        console.error('Dashboard: data is required');
        container.appendChild(createEmptyState());
        return;
      }
  
      options = options || {};
  
      // Clear container
      container.innerHTML = '';
  
      // Add header
      container.appendChild(createHeader(
        data.period,
        options.availablePeriods,
        options.onPeriodChange
      ));
  
      // Check if we have marketplaces
      const marketplaces = data.marketplaces || [];
      if (marketplaces.length === 0) {
        container.appendChild(createEmptyState());
        return;
      }
  
      // Check if we have rows
      const rows = data.rows || [];
      if (rows.length === 0) {
        container.appendChild(createEmptyState());
        return;
      }
  
      // Create table
      const tableWrapper = document.createElement('div');
      tableWrapper.className = 'dashboard-table-wrapper';
  
      const table = document.createElement('table');
      table.className = 'dashboard-table';
  
      // Sort handler
      function handleSort(column) {
        if (currentSortColumn === column) {
          currentSortDirection = currentSortDirection === 'desc' ? 'asc' : 'desc';
        } else {
          currentSortColumn = column;
          currentSortDirection = 'desc';
        }
  
        // Re-render table body
        const oldTbody = table.querySelector('tbody');
        const newTbody = createTableBody(marketplaces, rows, currentSortColumn, currentSortDirection);
        table.replaceChild(newTbody, oldTbody);
  
        // Update sort indicators
        updateSortIndicators(table, currentSortColumn, currentSortDirection);
      }
  
      table.appendChild(createTableHeader(marketplaces, handleSort));
      table.appendChild(createTableBody(marketplaces, rows, currentSortColumn, currentSortDirection));
  
      tableWrapper.appendChild(table);
      container.appendChild(tableWrapper);
    }
  
    // Export render function globally
    window.DashboardD400 = {
      render: render
    };
  
    // Also export as standalone function for backward compatibility
    window.render = render;
  })();
  
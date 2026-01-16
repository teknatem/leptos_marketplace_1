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
  
      marketplaces.forEach(function(marketplace) {
        const th = document.createElement('th');
        th.className = 'numeric sortable';
        th.dataset.column = marketplace;
  
        const content = document.createElement('span');
        content.textContent = marketplace;
        th.appendChild(content);
  
        const arrow = document.createElement('span');
        arrow.className = 'sort-arrow';
        th.appendChild(arrow);
  
        th.addEventListener('click', function() {
          onSort(marketplace);
        });
  
        tr.appendChild(th);
      });
  
      const thTotal = document.createElement('th');
      thTotal.className = 'numeric sortable';
      thTotal.dataset.column = 'total';
  
      const content = document.createElement('span');
      content.textContent = 'Итого';
      thTotal.appendChild(content);
  
      const arrow = document.createElement('span');
      arrow.className = 'sort-arrow';
      thTotal.appendChild(arrow);
  
      thTotal.addEventListener('click', function() {
        onSort('total');
      });
  
      tr.appendChild(thTotal);
  
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
  
      indicatorIds.forEach(function(indicatorId, index) {
        const indicatorRows = grouped[indicatorId];
  
        // Find level-0 row (total) for percentage calculations
        const totalRow = indicatorRows.find(function(r) { return r.level === 0; });
  
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
  
          const tdLabel = document.createElement('td');
          tdLabel.className = 'label';
  
          if (row.level === 0 || !row.group_name) {
            tdLabel.textContent = row.indicator_name || indicatorId;
          } else {
            tdLabel.textContent = row.group_name;
          }
  
          tr.appendChild(tdLabel);
  
          marketplaces.forEach(function(marketplace) {
            const td = document.createElement('td');
            td.className = 'numeric';
            const value = getValue(row.values, marketplace);
            td.innerHTML = formatNumber(value);
  
            // Add percentage for level-1 rows
            if (row.level === 1 && totalRow) {
              const totalValue = getValue(totalRow.values, marketplace);
              if (totalValue > 0) {
                const percentage = calculatePercentage(value, totalValue);
                const percentSpan = document.createElement('span');
                percentSpan.className = 'percentage';
                percentSpan.textContent = ' (' + formatPercentage(percentage) + ')';
                td.appendChild(percentSpan);
              }
            }
  
            tr.appendChild(td);
          });
  
          const tdTotal = document.createElement('td');
          tdTotal.className = 'numeric';
          const totalValue = getValue(row.values, 'total');
          tdTotal.innerHTML = formatNumber(totalValue);
  
          // Add percentage for level-1 rows
          if (row.level === 1 && totalRow) {
            const totalTotalValue = getValue(totalRow.values, 'total');
            if (totalTotalValue > 0) {
              const percentage = calculatePercentage(totalValue, totalTotalValue);
              const percentSpan = document.createElement('span');
              percentSpan.className = 'percentage';
              percentSpan.textContent = ' (' + formatPercentage(percentage) + ')';
              tdTotal.appendChild(percentSpan);
            }
          }
  
          tr.appendChild(tdTotal);
  
          tbody.appendChild(tr);
        });
  
        // Add separator between indicators (except after last one)
        if (index < indicatorIds.length - 1) {
          const separator = document.createElement('tr');
          separator.className = 'indicator-separator';
          const td = document.createElement('td');
          td.colSpan = marketplaces.length + 2;
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
  
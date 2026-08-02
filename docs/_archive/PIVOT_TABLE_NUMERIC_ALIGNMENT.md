# Выравнивание числовых значений в таблице результатов

## Реализация

Добавлено выравнивание числовых значений по правому краю в таблице результатов universal_dashboard.

## Что было сделано

### 1. Выравнивание заголовков числовых колонок

- Заголовки колонок с типом `ColumnType::Aggregated` (агрегированные числовые показатели) выравниваются по правому краю
- Используется `justify-content: flex-end;` для flex-контейнера заголовка

### 2. Выравнивание значений в ячейках

- Все ячейки с числовыми значениями (`CellValue::Number`, `CellValue::Integer`) выравниваются по правому краю
- Все ячейки в колонках типа `Aggregated` выравниваются по правому краю
- Стиль `text-align: right;` применяется к `<TableCell>`, что прижимает контент `.thaw-table-cell-layout__content` к правому краю `.thaw-table-cell-layout`

## Технические детали

### Определение числовых колонок

```rust
// Также проверяется тип колонки
let is_numeric_column = col_type == ColumnType::Aggregated || is_numeric;
```

### Стили для ячеек

```rust
// Style for TableCell to align content to the right
let cell_style = if is_numeric_column {
    "text-align: right;"
} else {
    ""
};

// Apply to TableCell component
<TableCell attr:style=cell_style>
    <TableCellLayout>
        <span style=span_style>{value}</span>
    </TableCellLayout>
</TableCell>
```

### Стили для заголовков

```rust
let header_align = if col_type == ColumnType::Aggregated {
    "justify-content: flex-end;"
} else {
    ""
};
```

## Типы колонок

В `ColumnType` есть два варианта:

- `Grouping` - колонки группировки (текстовые), выравнивание по левому краю
- `Aggregated` - агрегированные числовые колонки, выравнивание по правому краю

## Как это работает с компонентами Thaw

Thaw компонент `TableCell` создает следующую структуру DOM:

```html
<td class="thaw-table-cell" style="text-align: right;">
  <div class="thaw-table-cell-layout">
    <div class="thaw-table-cell-layout__content">
      <span>1 234.56</span>
    </div>
  </div>
</td>
```

Когда мы применяем `attr:style="text-align: right;"` к `<TableCell>`:

1. Устанавливается `text-align: right;` на элемент `<td>`
2. Это заставляет содержимое `.thaw-table-cell-layout__content` прижаться к правому краю родителя `.thaw-table-cell-layout`
3. Числа визуально выравниваются по правому краю ячейки таблицы

## Примеры

### До изменений:

```
Название       | Сумма       | Количество
----------------|-------------|------------
Товар 1        | 1234.56     | 10
Итого          | 5678.90     | 25
```

### После изменений:

```
Название       |       Сумма | Количество
----------------|-------------|------------
Товар 1        |     1234.56 |         10
Итого          |     5678.90 |         25
```

## Измененный файл

- `crates/frontend/src/shared/universal_dashboard/ui/pivot_table.rs`

## Преимущества

✅ **Лучшая читаемость** - числа легче сравнивать, когда они выровнены по правому краю  
✅ **Соответствие стандартам** - общепринятая практика для числовых данных  
✅ **Визуальная группировка** - разряды чисел выровнены вертикально  
✅ **Профессиональный вид** - таблица выглядит как в Excel/Google Sheets

## Совместимость

Работает со всеми типами числовых значений:

- Целые числа (`Integer`)
- Дробные числа (`Number`)
- Форматированные числа с разделителями тысяч
- Отрицательные числа
- Итоговые строки (сохраняют выравнивание)

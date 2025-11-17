# UI Updates: u501 - Флаг удаления устаревших записей

## Дата: 16.11.2025

## Описание

Добавлен чекбокс для опции удаления устаревших записей в UI импорта u501.

## Изменения

### Frontend UI (`crates/frontend/src/usecases/u501_import_from_ut/view.rs`)

1. **Добавлен новый signal:**

   ```rust
   let (delete_obsolete, set_delete_obsolete) = signal(false);
   ```

2. **Добавлена секция "Опции импорта" с чекбоксом:**

   - Расположена после списка агрегатов
   - Желтый фон с предупреждением (background: #fff3cd)
   - Чекбокс для включения/выключения удаления устаревших записей
   - Предупреждающее сообщение о жестком удалении

3. **Обновлен ImportRequest:**
   ```rust
   let request = ImportRequest {
       connection_id: conn_id,
       target_aggregates: targets,
       mode: ImportMode::Interactive,
       delete_obsolete: delete_obsolete.get(), // Добавлено
   };
   ```

## UI Элемент

```html
<div style="margin: 20px 0;">
  <label style="display: block; margin-bottom: 8px; font-weight: bold;">
    "Опции импорта:"
  </label>
  <div
    style="padding: 8px; background: #fff3cd; border-radius: 4px; border: 1px solid #ffc107;"
  >
    <label>
      <input
        type="checkbox"
        prop:checked="move"
        ||
        delete_obsolete.get()
        on:change="move"
        |ev|
        {
        set_delete_obsolete.set(event_target_checked(&ev));
        }
      />
      " Удалять устаревшие записи (которых нет в 1С)"
    </label>
    <div style="margin-top: 5px; font-size: 12px; color: #856404;">
      "⚠️ Внимание: Записи, которых нет в источнике 1С, будут удалены из БД
      (жесткое удаление)"
    </div>
  </div>
</div>
```

## Поведение

- **По умолчанию:** Флаг выключен (`delete_obsolete = false`)
- **При включении:** Все записи в БД, которых нет в источнике 1С, будут удалены жестко (DELETE)
- **Предупреждение:** Пользователь видит предупреждающее сообщение о необратимости операции

## Визуальное оформление

- **Цвета:** Желтый фон (#fff3cd) с оранжевой рамкой (#ffc107)
- **Иконка:** ⚠️ для привлечения внимания
- **Расположение:** Между списком агрегатов и кнопкой запуска импорта

## Связанные изменения

См. также изменения в:

- Backend: `crates/backend/src/usecases/u501_import_from_ut/executor.rs`
- Contracts: `crates/contracts/src/usecases/u501_import_from_ut/request.rs`
- Repository: `crates/backend/src/domain/a004_nomenclature/repository.rs`

## Тестирование

1. Открыть страницу импорта u501
2. Проверить отображение новой секции "Опции импорта"
3. Включить/выключить чекбокс
4. Запустить импорт с включенным флагом
5. Проверить логи на предмет удаления устаревших записей

## Примечания

- Флаг применяется ко всем выбранным агрегатам для импорта
- В текущей реализации удаление работает только для a004_nomenclature
- Рекомендуется использовать с осторожностью в production

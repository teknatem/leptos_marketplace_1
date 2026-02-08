//! Fields tab - detailed list of all schema fields

use crate::shared::list_utils::{get_sort_class, get_sort_indicator, Sortable};
use contracts::shared::universal_dashboard::{DataSourceSchemaOwned, FieldDefOwned, ValueType};
use leptos::prelude::*;
use std::cmp::Ordering;
use thaw::*;

/// Format value type for display (simplify Ref variant)
fn format_value_type(value_type: &ValueType) -> String {
    match value_type {
        ValueType::Ref { .. } => "Ref".to_string(),
        other => format!("{:?}", other),
    }
}

/// Get badge color for value type
fn get_value_type_color(value_type: &ValueType) -> BadgeColor {
    match value_type {
        ValueType::Integer => BadgeColor::Informative,
        ValueType::Numeric => BadgeColor::Success,
        ValueType::Text => BadgeColor::Brand,
        ValueType::Date => BadgeColor::Warning,
        ValueType::DateTime => BadgeColor::Important,
        ValueType::Boolean => BadgeColor::Danger,
        ValueType::Ref { .. } => BadgeColor::Subtle,
    }
}

impl Sortable for FieldDefOwned {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "id" => self.id.cmp(&other.id),
            "name" => self.name.cmp(&other.name),
            "value_type" => format_value_type(&self.get_value_type())
                .cmp(&format_value_type(&other.get_value_type())),
            "db_column" => self.db_column.cmp(&other.db_column),
            "can_group" => self.can_group.cmp(&other.can_group),
            "can_aggregate" => self.can_aggregate.cmp(&other.can_aggregate),
            _ => Ordering::Equal,
        }
    }
}

/// Fields tab component
#[component]
pub fn FieldsTab(schema: DataSourceSchemaOwned) -> impl IntoView {
    // States for sorting
    let (raw_fields, _set_raw_fields) = signal(schema.fields.clone());
    let (sorted_fields, set_sorted_fields) = signal(schema.fields.clone());
    let (sort_field, set_sort_field) = signal::<String>("id".to_string());
    let (sort_ascending, set_sort_ascending) = signal(true);

    // Auto-sort effect
    Effect::new(move |_| {
        let mut fields = raw_fields.get();
        let field = sort_field.get();
        let ascending = sort_ascending.get();

        fields.sort_by(|a, b| {
            let cmp = a.compare_by_field(b, &field);
            if ascending {
                cmp
            } else {
                cmp.reverse()
            }
        });

        set_sorted_fields.set(fields);
    });

    let toggle_sort = move |field: &'static str| {
        if sort_field.get() == field {
            set_sort_ascending.update(|a| *a = !*a);
        } else {
            set_sort_field.set(field.to_string());
            set_sort_ascending.set(true);
        }
    };
    view! {
        <Flex vertical=true gap=FlexGap::Medium>
            <div style="font-size: var(--font-size-lg); font-weight: 600; color: var(--color-text-primary);">
                "Список полей схемы"
            </div>

            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHeaderCell>
                            <span
                                style="cursor: pointer; user-select: none; display: inline-flex; align-items: center; gap: 4px;"
                                on:click=move |_| toggle_sort("id")
                            >
                                "ID поля"
                                <span class=move || get_sort_class(&sort_field.get(), "id")>
                                    {move || get_sort_indicator(&sort_field.get(), "id", sort_ascending.get())}
                                </span>
                            </span>
                        </TableHeaderCell>
                        <TableHeaderCell>
                            <span
                                style="cursor: pointer; user-select: none; display: inline-flex; align-items: center; gap: 4px;"
                                on:click=move |_| toggle_sort("name")
                            >
                                "Название"
                                <span class=move || get_sort_class(&sort_field.get(), "name")>
                                    {move || get_sort_indicator(&sort_field.get(), "name", sort_ascending.get())}
                                </span>
                            </span>
                        </TableHeaderCell>
                        <TableHeaderCell>
                            <span
                                style="cursor: pointer; user-select: none; display: inline-flex; align-items: center; gap: 4px;"
                                on:click=move |_| toggle_sort("value_type")
                            >
                                "Тип значения"
                                <span class=move || get_sort_class(&sort_field.get(), "value_type")>
                                    {move || get_sort_indicator(&sort_field.get(), "value_type", sort_ascending.get())}
                                </span>
                            </span>
                        </TableHeaderCell>
                        <TableHeaderCell>
                            <span
                                style="cursor: pointer; user-select: none; display: inline-flex; align-items: center; gap: 4px;"
                                on:click=move |_| toggle_sort("db_column")
                            >
                                "Колонка БД"
                                <span class=move || get_sort_class(&sort_field.get(), "db_column")>
                                    {move || get_sort_indicator(&sort_field.get(), "db_column", sort_ascending.get())}
                                </span>
                            </span>
                        </TableHeaderCell>
                        <TableHeaderCell>
                            <span
                                style="cursor: pointer; user-select: none; display: inline-flex; align-items: center; gap: 4px;"
                                on:click=move |_| toggle_sort("can_group")
                            >
                                "Группировка"
                                <span class=move || get_sort_class(&sort_field.get(), "can_group")>
                                    {move || get_sort_indicator(&sort_field.get(), "can_group", sort_ascending.get())}
                                </span>
                            </span>
                        </TableHeaderCell>
                        <TableHeaderCell>
                            <span
                                style="cursor: pointer; user-select: none; display: inline-flex; align-items: center; gap: 4px;"
                                on:click=move |_| toggle_sort("can_aggregate")
                            >
                                "Агрегация"
                                <span class=move || get_sort_class(&sort_field.get(), "can_aggregate")>
                                    {move || get_sort_indicator(&sort_field.get(), "can_aggregate", sort_ascending.get())}
                                </span>
                            </span>
                        </TableHeaderCell>
                        <TableHeaderCell>"Таблица источника"</TableHeaderCell>
                        <TableHeaderCell>"Ref таблица"</TableHeaderCell>
                        <TableHeaderCell>"Ref колонка"</TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {move || sorted_fields.get().into_iter().map(|field| {
                        let field_id = field.id.clone();
                        let field_name = field.name.clone();
                        let field_db_column = field.db_column.clone();
                        let value_type_obj = field.get_value_type();
                        let value_type = format_value_type(&value_type_obj);
                        let value_type_color = get_value_type_color(&value_type_obj);
                        let can_group = field.can_group;
                        let can_aggregate = field.can_aggregate;
                        let source_table = field.source_table.clone();
                        let ref_table = field.ref_table.clone();
                        let ref_display_column = field.ref_display_column.clone();

                        view! {
                            <TableRow>
                                <TableCell>
                                    <span style="font-family: var(--font-mono); font-size: var(--font-size-xs); color: var(--color-text-secondary);">
                                        {field_id}
                                    </span>
                                </TableCell>
                                <TableCell>
                                    <span style="font-weight: 500;">
                                        {field_name}
                                    </span>
                                </TableCell>
                                <TableCell>
                                    <Badge appearance=BadgeAppearance::Tint color=value_type_color>
                                        {value_type}
                                    </Badge>
                                </TableCell>
                                <TableCell>
                                    <span style="font-family: var(--font-mono); font-size: var(--font-size-xs);">
                                        {field_db_column}
                                    </span>
                                </TableCell>
                                <TableCell>
                                    {if can_group {
                                        view! {
                                            <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Success>
                                                "Да"
                                            </Badge>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Subtle>
                                                "Нет"
                                            </Badge>
                                        }.into_any()
                                    }}
                                </TableCell>
                                <TableCell>
                                    {if can_aggregate {
                                        view! {
                                            <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Success>
                                                "Да"
                                            </Badge>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Subtle>
                                                "Нет"
                                            </Badge>
                                        }.into_any()
                                    }}
                                </TableCell>
                                <TableCell>
                                    {source_table.as_ref().map(|t| view! {
                                        <span style="font-family: var(--font-mono); font-size: var(--font-size-xs);">
                                            {t.clone()}
                                        </span>
                                    })}
                                </TableCell>
                                <TableCell>
                                    {ref_table.as_ref().map(|t| view! {
                                        <span style="font-family: var(--font-mono); font-size: var(--font-size-xs);">
                                            {t.clone()}
                                        </span>
                                    })}
                                </TableCell>
                                <TableCell>
                                    {ref_display_column.as_ref().map(|c| view! {
                                        <span style="font-family: var(--font-mono); font-size: var(--font-size-xs);">
                                            {c.clone()}
                                        </span>
                                    })}
                                </TableCell>
                            </TableRow>
                        }
                    }).collect_view()}
                </TableBody>
            </Table>
        </Flex>
    }
}

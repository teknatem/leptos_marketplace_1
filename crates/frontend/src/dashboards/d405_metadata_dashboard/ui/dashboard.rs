use crate::shared::icons::icon;
use contracts::domain::a001_connection_1c::{
    ENTITY_METADATA as A001_ENTITY_METADATA, FIELDS as A001_FIELDS,
};
use contracts::shared::metadata::{EntityMetadataInfo, FieldMetadata};
use leptos::prelude::*;
use thaw::*;

#[derive(Clone, Copy)]
struct AggregateSource {
    key: &'static str,
    entity: EntityMetadataInfo,
    fields: &'static [FieldMetadata],
}

const AGGREGATES: &[AggregateSource] = &[AggregateSource {
    key: "a001_connection_1c",
    entity: A001_ENTITY_METADATA,
    fields: A001_FIELDS,
}];

type ExpandSet = std::collections::HashSet<String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TreeRoot {
    Aggregates,
    Usecases,
    Projections,
}

impl TreeRoot {
    fn id(&self) -> &'static str {
        match self {
            Self::Aggregates => "aggregates",
            Self::Usecases => "usecases",
            Self::Projections => "projections",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Aggregates => "Aggregates",
            Self::Usecases => "Usecases",
            Self::Projections => "Projections",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SelectedNode {
    Root {
        root: TreeRoot,
    },
    Aggregate {
        agg_key: &'static str,
    },
    Field {
        agg_key: &'static str,
        /// Path of field names from root aggregate (supports nested fields)
        path: Vec<&'static str>,
    },
}

fn opt_str(v: Option<&'static str>) -> &'static str {
    v.unwrap_or("—")
}

fn opt_u32(v: Option<u32>) -> String {
    v.map(|x| x.to_string()).unwrap_or_else(|| "—".to_string())
}

fn join_strs(v: &'static [&'static str]) -> String {
    if v.is_empty() {
        "—".to_string()
    } else {
        v.join(", ")
    }
}

fn join_path(path: &[&'static str]) -> String {
    path.join(".")
}

fn find_field_by_path(
    fields: &'static [FieldMetadata],
    path: &[&'static str],
) -> Option<FieldMetadata> {
    let (first, rest) = path.split_first()?;
    let field = fields.iter().copied().find(|f| f.name == *first)?;
    if rest.is_empty() {
        return Some(field);
    }
    find_field_by_path(field.nested_fields?, rest)
}

fn field_matches(field: &FieldMetadata, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    let q = q.to_lowercase();
    if field.name.to_lowercase().contains(&q) || field.ui.label.to_lowercase().contains(&q) {
        return true;
    }
    if let Some(nested) = field.nested_fields {
        return nested.iter().any(|f| field_matches(f, &q));
    }
    false
}

fn has_children(field: &FieldMetadata) -> bool {
    field.nested_fields.map(|n| !n.is_empty()).unwrap_or(false)
}

fn toggle_expand(expanded: RwSignal<ExpandSet>, node_id: String) {
    expanded.update(|set| {
        if set.contains(&node_id) {
            set.remove(&node_id);
        } else {
            set.insert(node_id);
        }
    });
}

fn is_expanded(expanded: RwSignal<ExpandSet>, node_id: &str, force_open: bool) -> bool {
    if force_open {
        true
    } else {
        expanded.get().contains(node_id)
    }
}

fn render_field_tree(
    agg_key: &'static str,
    field: FieldMetadata,
    level: usize,
    path: Vec<&'static str>,
    force_open: bool,
    selected: RwSignal<SelectedNode>,
    expanded: RwSignal<ExpandSet>,
    field_filter: RwSignal<String>,
) -> AnyView {
    let node_id = format!("field:{}:{}", agg_key, join_path(&path));
    let can_expand = has_children(&field);
    let expanded_now = is_expanded(expanded, &node_id, force_open);
    let padding_left = format!("{}px", 8 + level * 18);

    // For active styling and selection, capture a stable value
    let selected_value = SelectedNode::Field {
        agg_key,
        path: path.clone(),
    };
    let selected_value_for_active = selected_value.clone();
    let selected_value_for_click = selected_value.clone();

    let node_id_for_toggle = node_id.clone();
    let node_id_for_icon = node_id.clone();

    let children_view = if can_expand && expanded_now {
        let q = field_filter.get().trim().to_lowercase();
        let nested = field.nested_fields.unwrap_or(&[]);
        view! {
            <div>
                {nested
                    .iter()
                    .copied()
                    .filter(|f| force_open || field_matches(f, &q))
                    .map(|child| {
                        let mut child_path = path.clone();
                        child_path.push(child.name);
                        render_field_tree(
                            agg_key,
                            child,
                            level + 1,
                            child_path,
                            force_open,
                            selected,
                            expanded,
                            field_filter,
                        )
                    })
                    .collect_view()}
            </div>
        }
        .into_any()
    } else {
        view! { <></> }.into_any()
    };

    view! {
        <div>
            <div class="d401-tree__row" style:padding-left=padding_left>
                <Button
                    appearance=ButtonAppearance::Transparent
                    class="d401-tree__btn d401-tree__btn--field"
                    class:d401-tree__btn--active=move || selected.get() == selected_value_for_active
                    on_click=move |_| selected.set(selected_value_for_click.clone())
                >
                    <div class="d401-tree__btn-content">
                        <span
                            class="d401-tree__toggle"
                            class:d401-tree__toggle--disabled=move || !can_expand
                            on:click=move |ev| {
                                ev.stop_propagation();
                                if can_expand {
                                    toggle_expand(expanded, node_id_for_toggle.clone());
                                }
                            }
                        >
                            {move || {
                                if !can_expand {
                                    view! { <span class="d401-tree__toggle-placeholder"></span> }.into_any()
                                } else if is_expanded(expanded, &node_id_for_icon, force_open) {
                                    icon("chevron-down")
                                } else {
                                    icon("chevron-right")
                                }
                            }}
                        </span>

                        <span class="d401-tree__key">{field.name}</span>
                        <span class="d401-tree__label">{field.ui.label}</span>
                    </div>
                </Button>
            </div>
            {children_view}
        </div>
    }
    .into_any()
}

#[component]
pub fn MetadataDashboard() -> impl IntoView {
    let selected = RwSignal::new(SelectedNode::Aggregate {
        agg_key: AGGREGATES[0].key,
    });
    let expanded = RwSignal::new(ExpandSet::new());
    let field_filter = RwSignal::new(String::new());

    // Default expand Aggregates root
    expanded.update(|set| {
        set.insert("root:aggregates".to_string());
    });

    let current_aggregate = move || {
        let agg_key = match selected.get() {
            SelectedNode::Root { .. } => AGGREGATES[0].key,
            SelectedNode::Aggregate { agg_key } => agg_key,
            SelectedNode::Field { agg_key, .. } => agg_key,
        };
        AGGREGATES
            .iter()
            .copied()
            .find(|a| a.key == agg_key)
            .unwrap_or(AGGREGATES[0])
    };

    let current_field = move || match selected.get() {
        SelectedNode::Field { agg_key, ref path } => {
            let agg = AGGREGATES
                .iter()
                .copied()
                .find(|a| a.key == agg_key)
                .unwrap_or(AGGREGATES[0]);
            find_field_by_path(agg.fields, path)
        }
        _ => None,
    };

    let is_aggregate_selected = move || matches!(selected.get(), SelectedNode::Aggregate { .. });

    view! {
        <div id="d401_metadata_dashboard--dashboard" data-page-category="legacy" class="d401-root">
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center class="d401-header">
                <div>
                    <div class="d401-title">"Метаданные (POC)"</div>
                    <div class="d401-subtitle">"Aggregates → Fields • источник: contracts metadata_gen.rs"</div>
                </div>
                <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                    "d401_metadata_dashboard"
                </Badge>
            </Flex>

            <div class="d401-split">
                <div class="d401-left">
                    <div class="d401-panel">
                        <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center style="margin-bottom: 8px;">
                            <div class="d401-panel__title">"Tree"</div>
                            <Badge appearance=BadgeAppearance::Tint>
                                {move || current_aggregate().fields.len().to_string()}
                            </Badge>
                        </Flex>

                        <Input
                            value=field_filter
                            placeholder="Фильтр по name/label… (включая вложенные)"
                        />

                        <div class="d401-fields">
                            {move || {
                                let q_now = field_filter.get().trim().to_lowercase();
                                let force_open = !q_now.is_empty();

                                view! {
                                    {([TreeRoot::Aggregates, TreeRoot::Usecases, TreeRoot::Projections]).into_iter().map(|root| {
                                        let root_id = format!("root:{}", root.id());
                                        let root_selected = SelectedNode::Root { root };
                                        let root_selected_active = root_selected.clone();
                                        let root_selected_click = root_selected.clone();
                                        let root_expanded = is_expanded(expanded, &root_id, force_open);

                                        let _count = match root {
                                            TreeRoot::Aggregates => AGGREGATES.len(),
                                            TreeRoot::Usecases => 0usize,
                                            TreeRoot::Projections => 0usize,
                                        };

                                        view! {
                                            // Root row
                                            <div class="d401-tree__row" style:padding-left="6px">
                                                <Button
                                                    appearance=ButtonAppearance::Subtle
                                                    class="d401-tree__btn d401-tree__btn--root"
                                                    class:d401-tree__btn--active=move || selected.get() == root_selected_active
                                                    on_click=move |_| selected.set(root_selected_click.clone())
                                                >
                                                    <div class="d401-tree__btn-content">
                                                        <span
                                                            class="d401-tree__toggle"
                                                            on:click=move |ev| {
                                                                ev.stop_propagation();
                                                                toggle_expand(expanded, root_id.clone());
                                                            }
                                                        >
                                                            {move || if root_expanded { icon("chevron-down") } else { icon("chevron-right") }}
                                                        </span>
                                                        <span class="d401-tree__key">{root.id()}</span>
                                                        <span class="d401-tree__label">{root.label()}</span>
                                                    </div>
                                                </Button>
                                            </div>

                                            // Children rows
                                            {move || {
                                                if !root_expanded {
                                                    return view! { <></> }.into_any();
                                                }

                                                match root {
                                                    TreeRoot::Aggregates => {
                                                        let q = field_filter.get().trim().to_lowercase();
                                                        view! {
                                                            <div>
                                                                {AGGREGATES.iter().copied().filter(|a| {
                                                                    if q.is_empty() {
                                                                        true
                                                                    } else {
                                                                        a.key.to_lowercase().contains(&q)
                                                                            || a.entity.ui.list_name.to_lowercase().contains(&q)
                                                                            || a.fields.iter().any(|f| field_matches(f, &q))
                                                                    }
                                                                }).map(|a| {
                                                                    let agg_key = a.key;
                                                                    let agg_id = format!("agg:{}", agg_key);
                                                                    let agg_expanded = is_expanded(expanded, &agg_id, force_open);

                                                                    view! {
                                                                        <div class="d401-tree__row" style:padding-left="24px">
                                                                            <Button
                                                                                appearance=ButtonAppearance::Subtle
                                                                                class="d401-tree__btn"
                                                                                class:d401-tree__btn--active=move || selected.get() == SelectedNode::Aggregate { agg_key }
                                                                                on_click=move |_| selected.set(SelectedNode::Aggregate { agg_key })
                                                                            >
                                                                                <div class="d401-tree__btn-content">
                                                                                    <span
                                                                                        class="d401-tree__toggle"
                                                                                        on:click=move |ev| {
                                                                                            ev.stop_propagation();
                                                                                            toggle_expand(expanded, agg_id.clone());
                                                                                        }
                                                                                    >
                                                                                        {move || if agg_expanded { icon("chevron-down") } else { icon("chevron-right") }}
                                                                                    </span>
                                                                                    <span class="d401-tree__key">{agg_key}</span>
                                                                                    <span class="d401-tree__label">{a.entity.ui.list_name}</span>
                                                                                </div>
                                                                            </Button>
                                                                        </div>

                                                                        {move || {
                                                                            if !agg_expanded {
                                                                                return view! { <></> }.into_any();
                                                                            }

                                                                            let q2 = field_filter.get().trim().to_lowercase();
                                                                            view! {
                                                                                <div>
                                                                                    {a.fields.iter().copied().filter(|f| force_open || field_matches(f, &q2)).map(|f| {
                                                                                        render_field_tree(
                                                                                            agg_key,
                                                                                            f,
                                                                                            2,
                                                                                            vec![f.name],
                                                                                            force_open,
                                                                                            selected,
                                                                                            expanded,
                                                                                            field_filter,
                                                                                        )
                                                                                    }).collect_view()}
                                                                                </div>
                                                                            }.into_any()
                                                                        }}
                                                                    }
                                                                }).collect_view()}
                                                            </div>
                                                        }.into_any()
                                                    }
                                                    TreeRoot::Usecases => {
                                                        view! {
                                                            <div class="d401-tree__empty" style:padding-left="24px">
                                                                "Пока нет данных"
                                                            </div>
                                                        }.into_any()
                                                    }
                                                    TreeRoot::Projections => {
                                                        view! {
                                                            <div class="d401-tree__empty" style:padding-left="24px">
                                                                "Пока нет данных"
                                                            </div>
                                                        }.into_any()
                                                    }
                                                }
                                            }}
                                        }
                                    }).collect_view()}
                                }.into_any()
                            }}
                        </div>
                    </div>
                </div>

                <div class="d401-right">
                    {move || {
                        let agg = current_aggregate();
                        let e = agg.entity;
                        let selected = current_field();

                        view! {
                            <div class="d401-panel">
                                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                                    <div>
                                        <div class="d401-entity-title">{e.ui.list_name}</div>
                                        <div class="d401-entity-subtitle">
                                            {format!("{} • {} • table: {}", e.entity_index, e.entity_type.as_str(), opt_str(e.table_name))}
                                        </div>
                                    </div>
                                    <Space>
                                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Success>
                                            {e.collection_name}
                                        </Badge>
                                        <Badge appearance=BadgeAppearance::Tint>
                                            {format!("schema {}", e.schema_version)}
                                        </Badge>
                                    </Space>
                                </Flex>

                                <div class="d401-entity-desc">
                                    {e.ai.description}
                                </div>
                            </div>

                            <div class="d401-panel">
                                <div class="d401-panel__title">"Entity properties"</div>
                                <Table>
                                    <TableHeader>
                                        <TableRow>
                                            <TableHeaderCell attr:style="width: 220px;">"Key"</TableHeaderCell>
                                            <TableHeaderCell>"Value"</TableHeaderCell>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        <TableRow><TableCell>"entity_name"</TableCell><TableCell>{e.entity_name}</TableCell></TableRow>
                                        <TableRow><TableCell>"collection_name"</TableCell><TableCell>{e.collection_name}</TableCell></TableRow>
                                        <TableRow><TableCell>"table_name"</TableCell><TableCell>{opt_str(e.table_name)}</TableCell></TableRow>
                                        <TableRow><TableCell>"ui.element_name"</TableCell><TableCell>{e.ui.element_name}</TableCell></TableRow>
                                        <TableRow><TableCell>"ui.icon"</TableCell><TableCell>{opt_str(e.ui.icon)}</TableCell></TableRow>
                                        <TableRow><TableCell>"ai.questions"</TableCell><TableCell>{join_strs(e.ai.questions)}</TableCell></TableRow>
                                        <TableRow><TableCell>"ai.related"</TableCell><TableCell>{join_strs(e.ai.related)}</TableCell></TableRow>
                                    </TableBody>
                                </Table>
                            </div>

                            {selected.map(|f| view! {
                                <div class="d401-panel">
                                    <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center style="margin-bottom: 8px;">
                                        <div class="d401-panel__title">{format!("Field: {}", f.name)}</div>
                                        <Space>
                                            <Badge appearance=BadgeAppearance::Tint>{f.source.as_str()}</Badge>
                                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>{f.field_type.as_str()}</Badge>
                                        </Space>
                                    </Flex>

                                    <Table>
                                        <TableHeader>
                                            <TableRow>
                                                <TableHeaderCell attr:style="width: 220px;">"Key"</TableHeaderCell>
                                                <TableHeaderCell>"Value"</TableHeaderCell>
                                            </TableRow>
                                        </TableHeader>
                                        <TableBody>
                                            <TableRow><TableCell>"ui.label"</TableCell><TableCell>{f.ui.label}</TableCell></TableRow>
                                            <TableRow><TableCell>"rust_type"</TableCell><TableCell>{f.rust_type}</TableCell></TableRow>
                                            <TableRow><TableCell>"required"</TableCell><TableCell>{f.validation.required.to_string()}</TableCell></TableRow>
                                            <TableRow><TableCell>"visible_in_list"</TableCell><TableCell>{f.ui.visible_in_list.to_string()}</TableCell></TableRow>
                                            <TableRow><TableCell>"visible_in_form"</TableCell><TableCell>{f.ui.visible_in_form.to_string()}</TableCell></TableRow>
                                            <TableRow><TableCell>"widget"</TableCell><TableCell>{opt_str(f.ui.widget)}</TableCell></TableRow>
                                            <TableRow><TableCell>"column_width"</TableCell><TableCell>{opt_u32(f.ui.column_width)}</TableCell></TableRow>
                                            <TableRow><TableCell>"placeholder"</TableCell><TableCell>{opt_str(f.ui.placeholder)}</TableCell></TableRow>
                                            <TableRow><TableCell>"hint"</TableCell><TableCell>{opt_str(f.ui.hint)}</TableCell></TableRow>
                                            <TableRow><TableCell>"ai_hint"</TableCell><TableCell>{opt_str(f.ai_hint)}</TableCell></TableRow>
                                            <TableRow><TableCell>"ref_aggregate"</TableCell><TableCell>{opt_str(f.ref_aggregate)}</TableCell></TableRow>
                                        </TableBody>
                                    </Table>
                                </div>
                            })}

                            {move || {
                                if is_aggregate_selected() {
                                    view! {
                                        <div class="d401-panel">
                                            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center style="margin-bottom: 8px;">
                                                <div class="d401-panel__title">"All fields"</div>
                                                <Badge appearance=BadgeAppearance::Tint>
                                                    {agg.fields.len().to_string()}
                                                </Badge>
                                            </Flex>

                                            <div class="d401-table-scroll">
                                                <Table>
                                                    <TableHeader>
                                                        <TableRow>
                                                            <TableHeaderCell>"name"</TableHeaderCell>
                                                            <TableHeaderCell>"label"</TableHeaderCell>
                                                            <TableHeaderCell>"rust_type"</TableHeaderCell>
                                                            <TableHeaderCell>"type"</TableHeaderCell>
                                                            <TableHeaderCell>"source"</TableHeaderCell>
                                                            <TableHeaderCell>"required"</TableHeaderCell>
                                                            <TableHeaderCell>"list"</TableHeaderCell>
                                                            <TableHeaderCell>"form"</TableHeaderCell>
                                                            <TableHeaderCell>"widget"</TableHeaderCell>
                                                            <TableHeaderCell>"col_w"</TableHeaderCell>
                                                        </TableRow>
                                                    </TableHeader>
                                                    <TableBody>
                                                        {agg.fields.iter().map(|f| {
                                                            view! {
                                                                <TableRow>
                                                                    <TableCell>{f.name}</TableCell>
                                                                    <TableCell>{f.ui.label}</TableCell>
                                                                    <TableCell>{f.rust_type}</TableCell>
                                                                    <TableCell>{f.field_type.as_str()}</TableCell>
                                                                    <TableCell>{f.source.as_str()}</TableCell>
                                                                    <TableCell>{f.validation.required.to_string()}</TableCell>
                                                                    <TableCell>{f.ui.visible_in_list.to_string()}</TableCell>
                                                                    <TableCell>{f.ui.visible_in_form.to_string()}</TableCell>
                                                                    <TableCell>{opt_str(f.ui.widget)}</TableCell>
                                                                    <TableCell>{opt_u32(f.ui.column_width)}</TableCell>
                                                                </TableRow>
                                                            }
                                                        }).collect_view()}
                                                    </TableBody>
                                                </Table>
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }
                            }}
                        }.into_any()
                    }}
                </div>
            </div>
        </div>
    }
}

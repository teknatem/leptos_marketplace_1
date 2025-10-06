use super::super::details::CounterpartyDetails;
use contracts::domain::a003_counterparty::aggregate::Counterparty;
use contracts::domain::common::AggregateId;
use leptos::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;

fn api_base() -> String {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return String::new(),
    };
    let location = window.location();
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    format!("{}//{}:3000", protocol, hostname)
}

async fn fetch_counterparties() -> Result<Vec<Counterparty>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/counterparty", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: Vec<Counterparty> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

#[derive(Clone)]
struct TreeNode {
    item: Counterparty,
    children: Vec<TreeNode>,
    expanded: RwSignal<bool>,
}

/// Правильное построение дерева: сначала группируем детей, потом строим узлы
fn build_tree(items: Vec<Counterparty>) -> Vec<TreeNode> {
    use contracts::domain::common::AggregateId;

    if items.is_empty() {
        return vec![];
    }

    // Создаем set всех существующих ID для проверки валидности parent_id
    let existing_ids: std::collections::HashSet<String> =
        items.iter().map(|item| item.base.id.as_string()).collect();

    // Группируем детей по parent_id
    let mut children_map: HashMap<Option<String>, Vec<Counterparty>> = HashMap::new();
    for item in items {
        let parent_id = item.parent_id.clone();

        // Если parent_id указан, но родителя нет в списке - считаем элемент корневым
        let normalized_parent = if let Some(ref pid) = parent_id {
            // 00000000-0000-0000-0000-000000000000 - пустой GUID из 1С, эквивалент NULL
            if pid == "00000000-0000-0000-0000-000000000000" {
                None
            } else if existing_ids.contains(pid) {
                Some(pid.clone())
            } else {
                None
            }
        } else {
            None
        };

        children_map
            .entry(normalized_parent)
            .or_insert_with(Vec::new)
            .push(item);
    }

    // Рекурсивная функция для построения узла со всеми его детьми
    fn build_node(
        item: Counterparty,
        children_map: &HashMap<Option<String>, Vec<Counterparty>>,
    ) -> TreeNode {
        let id = item.base.id.as_string();
        let children = children_map
            .get(&Some(id.clone()))
            .map(|kids| {
                kids.iter()
                    .map(|kid| build_node(kid.clone(), children_map))
                    .collect()
            })
            .unwrap_or_else(Vec::new);

        TreeNode {
            item,
            children,
            expanded: RwSignal::new(false),
        }
    }

    // Сортировка узлов
    fn sort_nodes(nodes: &mut Vec<TreeNode>) {
        nodes.sort_by(|a, b| match (a.item.is_folder, b.item.is_folder) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a
                .item
                .base
                .description
                .to_lowercase()
                .cmp(&b.item.base.description.to_lowercase()),
        });
        for n in nodes.iter_mut() {
            if !n.children.is_empty() {
                sort_nodes(&mut n.children);
            }
        }
    }

    // Строим корневые узлы (без parent_id или с несуществующим parent_id)
    let mut roots = children_map
        .get(&None)
        .map(|root_items| {
            root_items
                .iter()
                .map(|item| build_node(item.clone(), &children_map))
                .collect()
        })
        .unwrap_or_else(Vec::new);

    sort_nodes(&mut roots);
    roots
}

/// Фильтрация дерева: возвращает узлы, соответствующие фильтру (рекурсивно)
fn filter_tree(nodes: Vec<TreeNode>, filter: &str) -> Vec<TreeNode> {
    if filter.trim().is_empty() {
        return nodes;
    }

    let filter_lower = filter.to_lowercase();
    let mut result = Vec::new();

    for node in nodes {
        let matches = node
            .item
            .base
            .description
            .to_lowercase()
            .contains(&filter_lower)
            || node.item.base.code.to_lowercase().contains(&filter_lower)
            || node.item.inn.to_lowercase().contains(&filter_lower)
            || node.item.kpp.to_lowercase().contains(&filter_lower);

        let filtered_children = filter_tree(node.children.clone(), filter);

        if matches || !filtered_children.is_empty() {
            let new_node = TreeNode {
                item: node.item.clone(),
                children: filtered_children,
                expanded: node.expanded,
            };
            // Авто-раскрываем узлы при фильтрации
            if !filter.trim().is_empty() && !new_node.children.is_empty() {
                new_node.expanded.set(true);
            }
            result.push(new_node);
        }
    }

    result
}

fn render_rows_with_lookup(
    node: TreeNode,
    level: usize,
    id_to_label: HashMap<String, String>,
    on_open: Rc<dyn Fn(String)>,
) -> Vec<AnyView> {
    let mut rows: Vec<AnyView> = Vec::new();

    let has_children = !node.children.is_empty();
    let expanded = node.expanded;
    let label = node.item.base.description.clone();
    let code = node.item.base.code.clone();
    let id = node.item.base.id.as_string();
    let is_folder = node.item.is_folder;
    let inn = node.item.inn.clone();
    let kpp = node.item.kpp.clone();
    let parent_label = node
        .item
        .parent_id
        .as_ref()
        .and_then(|pid| id_to_label.get(pid).cloned())
        .unwrap_or_else(|| "-".to_string());

    let toggle: AnyView = if is_folder {
        if has_children {
            view! { <button class="tree-toggle" on:click=move |_| expanded.update(|v| *v = !*v)>{move || if expanded.get() {"▼"} else {"▶"}}</button> }.into_any()
        } else {
            // Папка без детей - показываем неактивную стрелку
            view! { <span class="tree-toggle" style="color: #ccc; cursor: default;">{"▶"}</span> }
                .into_any()
        }
    } else {
        view! { <span class="tree-toggle-placeholder">{""}</span> }.into_any()
    };

    let open = {
        let on_open = on_open.clone();
        let id_clone = id.clone();
        move |_| (on_open)(id_clone.clone())
    };

    let row = view! {
        <tr class="tree-row">
            <td class="p-0-8">
                <div style={format!(
                    "display: flex; align-items: center; gap: 6px; padding-left: {}px;",
                    level * 16
                )}>
                    {toggle}
                    <span>{ if is_folder { "📂" } else { "📄" } }</span>
                    <span class="tree-label" on:click=open>
                        {label.clone()}
                    </span>
                </div>
            </td>
            <td>{code.clone()}</td>
            <td>{ if is_folder { "Да" } else { "Нет" } }</td>
            <td>{parent_label.clone()}</td>
            <td>{ if is_folder { String::new() } else { format!("{}{}{}", inn.clone(), if !inn.is_empty() && !kpp.is_empty() { " / " } else { "" }, kpp.clone()) } }</td>
        </tr>
    }
    .into_any();

    rows.push(row);

    if expanded.get() {
        for child in node.children.clone().into_iter() {
            let mut child_rows =
                render_rows_with_lookup(child, level + 1, id_to_label.clone(), on_open.clone());
            rows.append(&mut child_rows);
        }
    }

    rows
}

#[component]
pub fn CounterpartyTree() -> impl IntoView {
    let (all_roots, set_all_roots) = signal::<Vec<TreeNode>>(vec![]);
    let (error, set_error) = signal::<Option<String>>(None);
    let (show_modal, set_show_modal) = signal(false);
    let (editing_id, set_editing_id) = signal::<Option<String>>(None);
    let (id_to_label, set_id_to_label) = signal::<HashMap<String, String>>(HashMap::new());
    let (filter_text, set_filter_text) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);

    let load = move || {
        set_is_loading.set(true);
        set_error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_counterparties().await {
                Ok(list) => {
                    let map: HashMap<String, String> = list
                        .iter()
                        .map(|c| (c.base.id.as_string(), c.base.description.clone()))
                        .collect();
                    set_id_to_label.set(map);
                    let tree = build_tree(list);
                    set_all_roots.set(tree);
                    set_error.set(None);
                    set_is_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(format!("Ошибка загрузки: {}", e)));
                    set_is_loading.set(false);
                }
            }
        });
    };

    // Вычисляемое значение для отфильтрованного дерева
    let filtered_roots = move || {
        let roots = all_roots.get();
        let filter = filter_text.get();
        filter_tree(roots, &filter)
    };

    load();

    view! {
        <div class="content">
            <div class="header">
                <h2>{"Контрагенты"}</h2>
                <div class="header-actions">
                    <button class="btn btn-primary" on:click=move |_| { set_editing_id.set(None); set_show_modal.set(true); }>
                        {"➕ Новый"}
                    </button>
                    <button class="btn btn-secondary" on:click=move |_| load()>
                        {"🔄 Обновить"}
                    </button>
                </div>
            </div>
            {move || error.get().map(|e| view! { <div class="error" style="background: #fee; color: #c33; padding: 12px; border-radius: 4px; margin: 12px 0;">{e}</div> })}

            {move || if is_loading.get() {
                view! { <div style="text-align: center; padding: 20px; color: #666;">{"⏳ Загрузка..."}</div> }.into_any()
            } else {
                view! {
                    <>
                        <div style="margin-top: 12px; margin-bottom: 8px;">
                            <input
                                type="text"
                                placeholder="Поиск по наименованию, коду, ИНН или КПП..."
                                style="width: 100%; padding: 8px 12px; border: 1px solid #ddd; border-radius: 4px; font-size: 14px;"
                                prop:value=move || filter_text.get()
                                on:input=move |ev| set_filter_text.set(event_target_value(&ev))
                            />
                        </div>

                        <div class="table-container">
                            <table>
                                <thead>
                                    <tr class="text-left" style="border-bottom: 1px solid #ddd;">
                                        <th class="th-w-46p p-6-8">{"Наименование"}</th>
                                        <th class="th-w-14p p-6-8">{"Код"}</th>
                                        <th class="th-w-10p p-6-8">{"Папка"}</th>
                                        <th class="th-w-15p p-6-8">{"Родитель"}</th>
                                        <th class="th-w-15p p-6-8">{"ИНН / КПП"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {move || {
                                        let lookup = id_to_label.get();
                                        let roots = filtered_roots();
                                        if roots.is_empty() {
                                            let all_count = all_roots.get().len();
                                            let msg = if all_count == 0 {
                                                "Нет данных. Нажмите 'Обновить' или загрузите данные через импорт из 1С."
                                            } else {
                                                "По фильтру ничего не найдено"
                                            };
                                            view! { <tr><td colspan="5" class="text-center" style="color: #888; padding: 20px;">{msg}</td></tr> }.into_any()
                                        } else {
                                            let all_rows = roots
                                                .into_iter()
                                                .flat_map(move |n| render_rows_with_lookup(n, 0, lookup.clone(), Rc::new(move |id: String| { set_editing_id.set(Some(id)); set_show_modal.set(true); })))
                                                .collect::<Vec<_>>();
                                            all_rows.into_view().into_any()
                                        }
                                    }}
                                </tbody>
                            </table>
                        </div>
                    </>
                }.into_any()
            }}

            {move || if show_modal.get() {
                view! {
                    <div class="modal-overlay">
                        <div class="modal-content">
                            <CounterpartyDetails
                                id=editing_id.get()
                                on_saved=Rc::new(move |_| { set_show_modal.set(false); set_editing_id.set(None); load(); })
                                on_cancel=Rc::new(move |_| { set_show_modal.set(false); set_editing_id.set(None); })
                            />
                        </div>
                    </div>
                }.into_any()
            } else { view! { <></> }.into_any() }}
        </div>
    }
}

use contracts::system::s3::{S3FileCategory, S3FileDto};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cmp::Ordering;
use std::collections::HashSet;
use thaw::*;
use wasm_bindgen::JsCast;

use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::{
    TableCellCheckbox, TableCrosshairHighlight, TableHeaderCheckbox,
};
use crate::shared::date_utils::{format_bytes_compact, format_datetime};
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use crate::shared::modal_frame::ModalFrame;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_SYSTEM;
use crate::shared::table_utils::init_column_resize;
use crate::system::auth::guard::RequireAdmin;
use crate::system::s3::api;

const TABLE_ID: &str = "sys-s3-files-table";
const COLUMN_WIDTHS_KEY: &str = "sys_s3_files_column_widths";

#[component]
pub fn S3FilesPage() -> impl IntoView {
    view! {
        <RequireAdmin>
            <S3FilesList />
        </RequireAdmin>
    }
}

#[component]
fn S3FilesList() -> impl IntoView {
    let files: RwSignal<Vec<S3FileDto>> = RwSignal::new(Vec::new());
    let selected_ids: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());
    let filter_category = RwSignal::new(String::new());
    let search_query = RwSignal::new(String::new());
    let sort_field = RwSignal::new("created_at".to_string());
    let sort_ascending = RwSignal::new(false);
    let page = RwSignal::new(0usize);
    let page_size = RwSignal::new(100usize);
    let show_upload_dialog = RwSignal::new(false);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_filter_expanded, set_is_filter_expanded) = signal(true);

    let reload = Callback::new(move |_| {
        let category_value = filter_category.get_untracked();
        set_loading.set(true);
        set_error.set(None);
        spawn_local(async move {
            match api::fetch_files(category_from_value(&category_value)).await {
                Ok(response) => {
                    files.set(response.items);
                    selected_ids.update(|selected| selected.clear());
                    page.set(0);
                }
                Err(err) => set_error.set(Some(err)),
            }
            set_loading.set(false);
        });
    });

    Effect::new(move |_| {
        reload.run(());
    });

    let resize_initialized = StoredValue::new(false);
    Effect::new(move |_| {
        if !resize_initialized.get_value() {
            resize_initialized.set_value(true);
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(100).await;
                init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
            });
        }
    });

    let filtered_files = Signal::derive(move || {
        let query = search_query.get().trim().to_lowercase();
        let mut rows: Vec<S3FileDto> = files
            .get()
            .into_iter()
            .filter(|file| {
                query.is_empty()
                    || file.original_filename.to_lowercase().contains(&query)
                    || file.object_key.to_lowercase().contains(&query)
                    || file.bucket.to_lowercase().contains(&query)
                    || file
                        .content_type
                        .as_deref()
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&query)
                    || file
                        .uploaded_by_user_id
                        .as_deref()
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&query)
            })
            .collect();

        let field = sort_field.get();
        let ascending = sort_ascending.get();
        rows.sort_by(|left, right| compare_files(left, right, &field, ascending));
        rows
    });

    let total_count = Signal::derive(move || filtered_files.get().len());
    let total_pages = Signal::derive(move || {
        let count = total_count.get();
        if count == 0 {
            0
        } else {
            count.div_ceil(page_size.get().max(1))
        }
    });
    let paged_files = Signal::derive(move || {
        let rows = filtered_files.get();
        let size = page_size.get().max(1);
        let start = page.get().saturating_mul(size).min(rows.len());
        let end = (start + size).min(rows.len());
        rows[start..end].to_vec()
    });

    let active_filters_count = Signal::derive(move || {
        let mut count = 0;
        if !filter_category.get().is_empty() {
            count += 1;
        }
        if !search_query.get().trim().is_empty() {
            count += 1;
        }
        count
    });

    Effect::new(move |_| {
        search_query.track();
        page.set(0);
    });

    Effect::new(move |_| {
        let total = total_pages.get();
        let current = page.get();
        if total > 0 && current >= total {
            page.set(total - 1);
        }
    });

    let toggle_sort = move |field: &'static str| {
        if sort_field.with_untracked(|current| current == field) {
            sort_ascending.update(|value| *value = !*value);
        } else {
            sort_field.set(field.to_string());
            sort_ascending.set(true);
        }
        page.set(0);
    };

    let go_to_page = move |new_page: usize| {
        let total = total_pages.get();
        if total == 0 {
            page.set(0);
        } else {
            page.set(new_page.min(total - 1));
        }
    };

    let change_page_size = move |new_size: usize| {
        page_size.set(new_size.max(1));
        page.set(0);
    };

    let toggle_selection = move |(id, checked): (String, bool)| {
        selected_ids.update(|selected| {
            if checked {
                selected.insert(id);
            } else {
                selected.remove(&id);
            }
        });
    };

    let toggle_all = move |check_all: bool| {
        if check_all {
            let ids = paged_files
                .get()
                .into_iter()
                .map(|file| file.id)
                .collect::<HashSet<_>>();
            selected_ids.set(ids);
        } else {
            selected_ids.update(|selected| selected.clear());
        }
    };

    view! {
        <PageFrame page_id="sys_s3_files--system" category=PAGE_CAT_SYSTEM class="page--wide">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"S3 файлы"</h1>
                    <span class="badge badge--neutral">{move || total_count.get().to_string()}</span>
                    {move || {
                        let selected = selected_ids.get().len();
                        if selected > 0 {
                            view! { <span class="badge badge--primary">{format!("Выбрано: {}", selected)}</span> }.into_any()
                        } else {
                            view! { <></> }.into_any()
                        }
                    }}
                </div>
                <div class="page__actions">
                    <button class="button button--secondary" on:click=move |_| reload.run(()) disabled=loading>
                        {icon("refresh")} {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                    </button>
                </div>
            </div>

            <div class="page__content">
                <div class="filter-panel">
                    <div class="filter-panel-header">
                        <div
                            class="filter-panel-header__left"
                            on:click=move |_| set_is_filter_expanded.update(|expanded| *expanded = !*expanded)
                        >
                            {icon(if is_filter_expanded.get() { "chevron-down" } else { "chevron-right" })}
                            {icon("filter")}
                            <span class="filter-panel__title">"Фильтры и загрузка"</span>
                            {move || {
                                let count = active_filters_count.get();
                                if count > 0 {
                                    view! { <span class="filter-panel__badge">{count}</span> }.into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }
                            }}
                        </div>

                        <div class="filter-panel-header__center">
                            <PaginationControls
                                current_page=Signal::derive(move || page.get())
                                total_pages=total_pages
                                total_count=total_count
                                page_size=Signal::derive(move || page_size.get())
                                on_page_change=Callback::new(go_to_page)
                                on_page_size_change=Callback::new(change_page_size)
                                page_size_options=vec![50, 100, 200, 500]
                            />
                        </div>

                        <div class="filter-panel-header__right">
                            <button class="button button--primary" on:click=move |_| show_upload_dialog.set(true)>
                                {icon("upload")}
                                "Загрузить файл"
                            </button>
                        </div>
                    </div>

                    <Show when=move || is_filter_expanded.get()>
                        <div class="filter-panel-content">
                            <div style="display: flex; gap: 12px; align-items: end; flex-wrap: wrap;">
                                <div style="width: 220px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Категория:"</Label>
                                        <select
                                            class="form__select"
                                            prop:value=move || filter_category.get()
                                            on:change=move |ev| {
                                                filter_category.set(event_target_value(&ev));
                                                page.set(0);
                                                reload.run(());
                                            }
                                        >
                                            <option value="">"Все категории"</option>
                                            {S3FileCategory::all()
                                                .iter()
                                                .cloned()
                                                .map(|item| view! {
                                                    <option value=item.as_str()>{item.label_ru()}</option>
                                                })
                                                .collect_view()}
                                        </select>
                                    </Flex>
                                </div>

                                <div style="flex: 1; min-width: 260px; max-width: 420px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Поиск:"</Label>
                                        <Input
                                            value=search_query
                                            placeholder="Имя файла, bucket, key, тип, пользователь..."
                                        />
                                    </Flex>
                                </div>
                            </div>
                        </div>
                    </Show>
                </div>

                {move || {
                    error.get().map(|err| view! {
                        <div class="alert alert--error">{err}</div>
                    })
                }}

                <div class="table-wrapper">
                    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />

                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 1240px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCheckbox
                                    items=paged_files
                                    selected=Signal::derive(move || selected_ids.get())
                                    get_id=Callback::new(|row: S3FileDto| row.id)
                                    on_change=Callback::new(toggle_all)
                                />
                                <TableHeaderCell resizable=false min_width=260.0 class="resizable">
                                    {sortable_header("Файл", "original_filename", sort_field, sort_ascending, toggle_sort)}
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=140.0 class="resizable">
                                    {sortable_header("Категория", "category", sort_field, sort_ascending, toggle_sort)}
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    {sortable_header("Размер", "size_bytes", sort_field, sort_ascending, toggle_sort)}
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=170.0 class="resizable">
                                    {sortable_header("Тип", "content_type", sort_field, sort_ascending, toggle_sort)}
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=150.0 class="resizable">
                                    {sortable_header("Загружен", "created_at", sort_field, sort_ascending, toggle_sort)}
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=150.0 class="resizable">
                                    {sortable_header("Пользователь", "uploaded_by_user_id", sort_field, sort_ascending, toggle_sort)}
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=290.0 class="resizable">
                                    {sortable_header("S3 объект", "object_key", sort_field, sort_ascending, toggle_sort)}
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=112.0 class="resizable">
                                    "Действия"
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            {move || {
                                let rows = paged_files.get();
                                if rows.is_empty() {
                                    view! {
                                        <TableRow>
                                            <TableCell attr:colspan="9">
                                                <TableCellLayout>
                                                    {if loading.get() { "Загрузка списка файлов..." } else { "Файлы не найдены" }}
                                                </TableCellLayout>
                                            </TableCell>
                                        </TableRow>
                                    }.into_any()
                                } else {
                                    rows.into_iter()
                                        .map(|item| {
                                            render_file_row(
                                                item,
                                                selected_ids,
                                                Callback::new(toggle_selection),
                                                reload,
                                                set_error,
                                            )
                                        })
                                        .collect_view()
                                        .into_any()
                                }
                            }}
                        </TableBody>
                    </Table>
                </div>
            </div>

            <S3UploadDialog show=show_upload_dialog on_uploaded=reload />
        </PageFrame>
    }
}

#[component]
fn S3UploadDialog(show: RwSignal<bool>, on_uploaded: Callback<()>) -> impl IntoView {
    let upload_category = RwSignal::new(S3FileCategory::Documents.as_str().to_string());
    let selected_file = StoredValue::new_local(None::<web_sys::File>);
    let file_info = RwSignal::new(None::<(String, u64)>);
    let (uploading, set_uploading) = signal(false);
    let (result, set_result) = signal::<Option<Result<String, String>>>(None);

    let close = Callback::new(move |_: ()| {
        selected_file.set_value(None);
        file_info.set(None);
        set_result.set(None);
        upload_category.set(S3FileCategory::Documents.as_str().to_string());
        set_uploading.set(false);
        show.set(false);
    });

    let upload = move |_| {
        let Some(file) = selected_file.get_value() else {
            set_result.set(Some(Err("Выберите файл для загрузки".to_string())));
            return;
        };
        let file_name = file.name();
        let selected_category = S3FileCategory::from(upload_category.get_untracked().as_str());
        set_uploading.set(true);
        set_result.set(None);
        spawn_local(async move {
            match api::upload_file(selected_category, file).await {
                Ok(_) => {
                    set_result.set(Some(Ok(format!("Файл «{file_name}» загружен"))));
                    selected_file.set_value(None);
                    file_info.set(None);
                    on_uploaded.run(());
                }
                Err(err) => set_result.set(Some(Err(err))),
            }
            set_uploading.set(false);
        });
    };

    view! {
        <Show when=move || show.get() fallback=|| view! {}>
            <ModalFrame
                on_close=close
                modal_style="max-width: 520px; width: 92vw;".to_string()
            >
                <div class="modal-header">
                    <span class="modal-title">"Загрузка файла в S3"</span>
                </div>

                <div class="modal-body" style="display: flex; flex-direction: column; gap: 16px;">
                    <Flex vertical=true gap=FlexGap::Small>
                        <Label>"Категория:"</Label>
                        <select
                            class="form__select"
                            prop:value=move || upload_category.get()
                            on:change=move |ev| upload_category.set(event_target_value(&ev))
                        >
                            {S3FileCategory::all()
                                .iter()
                                .cloned()
                                .map(|item| view! {
                                    <option value=item.as_str()>{item.label_ru()}</option>
                                })
                                .collect_view()}
                        </select>
                    </Flex>

                    <Flex vertical=true gap=FlexGap::Small>
                        <Label>"Файл:"</Label>
                        <input
                            class="form__input form__input--file"
                            type="file"
                            on:change=move |ev| {
                                let file = ev
                                    .target()
                                    .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                                    .and_then(|input| input.files())
                                    .and_then(|list| list.get(0));
                                file_info.set(file.as_ref().map(|f| (f.name(), f.size() as u64)));
                                selected_file.set_value(file);
                                set_result.set(None);
                            }
                        />
                    </Flex>

                    {move || file_info.get().map(|(name, size)| view! {
                        <div class="text-muted" style="font-size: 13px;">
                            "Выбран файл: "
                            <span style="font-weight: 600;">{name}</span>
                            {format!(" ({})", format_bytes_compact(size))}
                        </div>
                    })}

                    {move || result.get().map(|outcome| match outcome {
                        Ok(message) => view! { <div class="alert alert--success">{message}</div> }.into_any(),
                        Err(err) => view! { <div class="alert alert--error">{err}</div> }.into_any(),
                    })}
                </div>

                <div class="modal-footer">
                    <button
                        class="button button--secondary"
                        on:click=move |_| close.run(())
                        disabled=uploading
                    >
                        "Закрыть"
                    </button>
                    <button
                        class="button button--primary"
                        on:click=upload
                        disabled=move || uploading.get() || file_info.get().is_none()
                    >
                        {icon("upload")}
                        {move || if uploading.get() { "Загрузка..." } else { "Загрузить" }}
                    </button>
                </div>
            </ModalFrame>
        </Show>
    }
}

fn sortable_header(
    label: &'static str,
    field: &'static str,
    sort_field: RwSignal<String>,
    sort_ascending: RwSignal<bool>,
    toggle_sort: impl Fn(&'static str) + Copy + 'static,
) -> impl IntoView {
    view! {
        <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort(field)>
            {label}
            <span class=move || sort_field.with(|current| get_sort_class(current, field))>
                {move || {
                    let current = sort_field.get();
                    get_sort_indicator(&current, field, sort_ascending.get())
                }}
            </span>
        </div>
    }
}

fn render_file_row(
    item: S3FileDto,
    selected_ids: RwSignal<HashSet<String>>,
    toggle_selection: Callback<(String, bool)>,
    reload: Callback<()>,
    set_error: WriteSignal<Option<String>>,
) -> impl IntoView {
    let item_id = item.id.clone();
    let download_id = item.id.clone();
    let download_name = item.original_filename.clone();
    let delete_id = item.id.clone();
    let delete_name = item.original_filename.clone();
    let category = item.category.clone();

    view! {
        <TableRow>
            <TableCellCheckbox
                item_id=item_id
                selected=Signal::derive(move || selected_ids.get())
                on_change=toggle_selection
            />

            <TableCell>
                <TableCellLayout truncate=true>
                    <div style="display: grid; gap: 2px; min-width: 0;">
                        <span style="font-weight: 600;">{item.original_filename.clone()}</span>
                        <span class="text-muted" style="font-size: 12px;">{item.id.clone()}</span>
                    </div>
                </TableCellLayout>
            </TableCell>

            <TableCell>
                <TableCellLayout>
                    {category_badge(category)}
                </TableCellLayout>
            </TableCell>

            <TableCell>
                <TableCellLayout>
                    {format_bytes_compact(item.size_bytes.max(0) as u64)}
                </TableCellLayout>
            </TableCell>

            <TableCell>
                <TableCellLayout truncate=true>
                    {item.content_type.clone().unwrap_or_else(|| "application/octet-stream".to_string())}
                </TableCellLayout>
            </TableCell>

            <TableCell>
                <TableCellLayout>
                    {format_datetime(&item.created_at)}
                </TableCellLayout>
            </TableCell>

            <TableCell>
                <TableCellLayout truncate=true>
                    {item.uploaded_by_user_id.clone().unwrap_or_else(|| "-".to_string())}
                </TableCellLayout>
            </TableCell>

            <TableCell>
                <TableCellLayout truncate=true>
                    <div style="display: grid; gap: 2px; min-width: 0;">
                        <span style="font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: 12px;">{item.bucket.clone()}</span>
                        <span class="text-muted" style="font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: 12px;">{item.object_key.clone()}</span>
                    </div>
                </TableCellLayout>
            </TableCell>

            <TableCell>
                <TableCellLayout>
                    <div style="display: flex; gap: 6px;">
                        <button
                            class="button button--small button--secondary"
                            title="Скачать"
                            on:click=move |_| {
                                let id = download_id.clone();
                                let name = download_name.clone();
                                spawn_local(async move {
                                    if let Err(err) = api::download_file(&id, &name).await {
                                        set_error.set(Some(err));
                                    }
                                });
                            }
                        >
                            {icon("download")}
                        </button>
                        <button
                            class="button button--small button--danger"
                            title="Удалить"
                            on:click=move |_| {
                                let confirmed = web_sys::window()
                                    .and_then(|window| window.confirm_with_message(&format!("Удалить файл \"{}\"?", delete_name)).ok())
                                    .unwrap_or(false);
                                if !confirmed {
                                    return;
                                }
                                let id = delete_id.clone();
                                spawn_local(async move {
                                    match api::delete_file(&id).await {
                                        Ok(()) => reload.run(()),
                                        Err(err) => set_error.set(Some(err)),
                                    }
                                });
                            }
                        >
                            {icon("trash")}
                        </button>
                    </div>
                </TableCellLayout>
            </TableCell>
        </TableRow>
    }
}

fn category_from_value(value: &str) -> Option<S3FileCategory> {
    if value.is_empty() {
        None
    } else {
        Some(S3FileCategory::from(value))
    }
}

fn category_badge(category: S3FileCategory) -> impl IntoView {
    let color = match category {
        S3FileCategory::Documents => BadgeColor::Brand,
        S3FileCategory::Plugins => BadgeColor::Success,
        S3FileCategory::Backups => BadgeColor::Warning,
        S3FileCategory::ConferenceAudio => BadgeColor::Important,
        S3FileCategory::Other => BadgeColor::Subtle,
    };

    view! {
        <Badge appearance=BadgeAppearance::Tint color=color>
            {category.label_ru()}
        </Badge>
    }
}

fn compare_files(left: &S3FileDto, right: &S3FileDto, field: &str, ascending: bool) -> Ordering {
    let ordering = match field {
        "original_filename" => cmp_str(&left.original_filename, &right.original_filename),
        "category" => left.category.as_str().cmp(right.category.as_str()),
        "size_bytes" => left.size_bytes.cmp(&right.size_bytes),
        "content_type" => cmp_opt_str(&left.content_type, &right.content_type),
        "created_at" => left.created_at.cmp(&right.created_at),
        "uploaded_by_user_id" => cmp_opt_str(&left.uploaded_by_user_id, &right.uploaded_by_user_id),
        "object_key" => cmp_str(&left.object_key, &right.object_key),
        _ => Ordering::Equal,
    };

    if ascending {
        ordering
    } else {
        ordering.reverse()
    }
}

fn cmp_str(left: &str, right: &str) -> Ordering {
    left.to_lowercase().cmp(&right.to_lowercase())
}

fn cmp_opt_str(left: &Option<String>, right: &Option<String>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => cmp_str(left, right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

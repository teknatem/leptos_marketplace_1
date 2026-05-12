use crate::layout::global_context::AppGlobalContext;
use leptos::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
enum LinkPart {
    Text(String),
    KbArticle { id: String, raw: String },
}

#[component]
pub fn KbLinkedText(text: String) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let parts = parse_kb_links(&text);

    view! {
        <span style="white-space: pre-wrap;">
            {parts.into_iter().map(move |part| {
                match part {
                    LinkPart::Text(value) => view! { <span>{value}</span> }.into_any(),
                    LinkPart::KbArticle { id, raw } => {
                        let id_for_click = id.clone();
                        view! {
                            <a
                                href="#"
                                class="table__link"
                                on:click=move |ev| {
                                    ev.prevent_default();
                                    tabs_store.open_tab(
                                        &format!("kb_article_{}", id_for_click),
                                        &format!("KB {}", id_for_click),
                                    );
                                }
                            >
                                {raw}
                            </a>
                        }.into_any()
                    }
                }
            }).collect_view()}
        </span>
    }
}

fn parse_kb_links(text: &str) -> Vec<LinkPart> {
    const PREFIX: &str = "kb://article/";
    let mut parts = Vec::new();
    let mut remaining = text;

    while let Some(start) = remaining.find(PREFIX) {
        if start > 0 {
            parts.push(LinkPart::Text(remaining[..start].to_string()));
        }

        let after_prefix = &remaining[start + PREFIX.len()..];
        let id_len = after_prefix
            .char_indices()
            .find_map(|(idx, ch)| {
                if is_link_terminator(ch) {
                    Some(idx)
                } else {
                    None
                }
            })
            .unwrap_or(after_prefix.len());

        if id_len == 0 {
            parts.push(LinkPart::Text(PREFIX.to_string()));
            remaining = after_prefix;
            continue;
        }

        let id = after_prefix[..id_len].to_string();
        let raw = format!("{}{}", PREFIX, id);
        parts.push(LinkPart::KbArticle { id, raw });
        remaining = &after_prefix[id_len..];
    }

    if !remaining.is_empty() {
        parts.push(LinkPart::Text(remaining.to_string()));
    }

    parts
}

fn is_link_terminator(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, ')' | ']' | '}' | ',' | ';' | '"' | '\'')
}

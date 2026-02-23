use super::stat_card::StatCard;
use contracts::shared::indicators::*;
use leptos::prelude::*;
use std::collections::HashMap;

#[component]
pub fn IndicatorSetView(
    /// Set metadata (label, indicator list, columns)
    set_meta: IndicatorSetMeta,
    /// Metadata for individual indicators
    indicator_metas: Vec<IndicatorMeta>,
    /// Computed values keyed by indicator id string
    #[prop(into)]
    values: Signal<HashMap<String, IndicatorValue>>,
) -> impl IntoView {
    let cols_class = match set_meta.columns {
        2 => "indicator-set__grid indicator-set__grid--cols-2",
        3 => "indicator-set__grid indicator-set__grid--cols-3",
        _ => "indicator-set__grid indicator-set__grid--cols-4",
    };

    let meta_map: HashMap<String, IndicatorMeta> = indicator_metas
        .into_iter()
        .map(|m| (m.id.0.clone(), m))
        .collect();

    let cards: Vec<_> = set_meta
        .indicators
        .iter()
        .filter_map(|ind_id| {
            let meta = meta_map.get(&ind_id.0)?.clone();
            let id_str = ind_id.0.clone();

            let value_sig = Signal::derive({
                let id_str = id_str.clone();
                move || {
                    values
                        .get()
                        .get(&id_str)
                        .and_then(|v| v.value)
                }
            });

            let status_sig = Signal::derive({
                let id_str = id_str.clone();
                move || {
                    values
                        .get()
                        .get(&id_str)
                        .map(|v| v.status)
                        .unwrap_or(IndicatorStatus::Neutral)
                }
            });

            let change_sig = Signal::derive({
                let id_str = id_str.clone();
                move || {
                    values
                        .get()
                        .get(&id_str)
                        .and_then(|v| v.change_percent)
                }
            });

            let subtitle_sig = Signal::derive({
                let id_str = id_str.clone();
                move || {
                    values
                        .get()
                        .get(&id_str)
                        .and_then(|v| v.subtitle.clone())
                }
            });

            Some(view! {
                <StatCard
                    label=meta.label.clone()
                    icon_name=meta.icon.clone()
                    value=value_sig
                    format=meta.format.clone()
                    status=status_sig
                    change_percent=change_sig
                    subtitle=subtitle_sig
                />
            })
        })
        .collect();

    view! {
        <div class="indicator-set">
            <div class="indicator-set__title">{set_meta.label}</div>
            <div class=cols_class>
                {cards}
            </div>
        </div>
    }
}

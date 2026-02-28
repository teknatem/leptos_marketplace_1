//! Dimensions tab - nomenclature dimension fields with autocomplete
//!
//! Contains all 6 dimension fields with DimensionInput component

use super::super::dimension_input::DimensionInput;
use super::super::view_model::NomenclatureDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;
use thaw::*;

/// Dimensions tab component with all dimension fields
#[component]
pub fn DimensionsTab(vm: NomenclatureDetailsVm) -> impl IntoView {
    view! {
        <CardAnimated delay_ms=0>
            <h4 class="details-section__title">"Измерения"</h4>
            <DimensionInput
                id="dim1_category"
                label="Категория"
                placeholder="Категория (макс. 40 символов)"
                maxlength=40
                value=Signal::derive({
                    let dim = vm.dim1_category;
                    move || dim.get()
                })
                on_change=Callback::new({
                    let dim = vm.dim1_category;
                    move |v| dim.set(v)
                })
                options=vm.get_dim_options("dim1_category")
            />

            <DimensionInput
                id="dim2_line"
                label="Линейка"
                placeholder="Линейка (макс. 40 символов)"
                maxlength=40
                value=Signal::derive({
                    let dim = vm.dim2_line;
                    move || dim.get()
                })
                on_change=Callback::new({
                    let dim = vm.dim2_line;
                    move |v| dim.set(v)
                })
                options=vm.get_dim_options("dim2_line")
            />

            <DimensionInput
                id="dim3_model"
                label="Модель"
                placeholder="Модель (макс. 80 символов)"
                maxlength=80
                value=Signal::derive({
                    let dim = vm.dim3_model;
                    move || dim.get()
                })
                on_change=Callback::new({
                    let dim = vm.dim3_model;
                    move |v| dim.set(v)
                })
                options=vm.get_dim_options("dim3_model")
            />

            <DimensionInput
                id="dim4_format"
                label="Формат"
                placeholder="Формат (макс. 20 символов)"
                maxlength=20
                value=Signal::derive({
                    let dim = vm.dim4_format;
                    move || dim.get()
                })
                on_change=Callback::new({
                    let dim = vm.dim4_format;
                    move |v| dim.set(v)
                })
                options=vm.get_dim_options("dim4_format")
            />

            <DimensionInput
                id="dim5_sink"
                label="Раковина"
                placeholder="Раковина (макс. 40 символов)"
                maxlength=40
                value=Signal::derive({
                    let dim = vm.dim5_sink;
                    move || dim.get()
                })
                on_change=Callback::new({
                    let dim = vm.dim5_sink;
                    move |v| dim.set(v)
                })
                options=vm.get_dim_options("dim5_sink")
            />

            <DimensionInput
                id="dim6_size"
                label="Размер"
                placeholder="Размер (макс. 20 символов)"
                maxlength=20
                value=Signal::derive({
                    let dim = vm.dim6_size;
                    move || dim.get()
                })
                on_change=Callback::new({
                    let dim = vm.dim6_size;
                    move |v| dim.set(v)
                })
                options=vm.get_dim_options("dim6_size")
            />
        </CardAnimated>
    }
}

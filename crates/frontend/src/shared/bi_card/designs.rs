#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndicatorDesign {
    pub key: &'static str,
    pub label: &'static str,
}

pub const CORE_DESIGNS: [IndicatorDesign; 4] = [
    IndicatorDesign {
        key: "classic",
        label: "Classic",
    },
    IndicatorDesign {
        key: "modern",
        label: "Modern",
    },
    IndicatorDesign {
        key: "retro",
        label: "Retro",
    },
    IndicatorDesign {
        key: "future",
        label: "Future",
    },
];

pub const CUSTOM_DESIGN: IndicatorDesign = IndicatorDesign {
    key: "custom",
    label: "Custom CSS",
};

pub fn default_design_name() -> &'static str {
    "classic"
}

pub fn available_designs(has_custom_css: bool) -> Vec<IndicatorDesign> {
    let mut out = CORE_DESIGNS.to_vec();
    if has_custom_css {
        out.push(CUSTOM_DESIGN);
    }
    out
}

pub fn is_known_design(design: &str, has_custom_css: bool) -> bool {
    available_designs(has_custom_css)
        .iter()
        .any(|entry| entry.key == design)
}

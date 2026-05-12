//! Shared types for navigator pages.

use serde::{Deserialize, Serialize};

/// Identifier of a marketplace column on a navigator page.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MarketplaceKind {
    Wildberries,
    Ozon,
    YandexMarket,
}

/// A single column of the marketplace matrix (one column = one marketplace).
#[derive(Clone, Copy)]
pub struct MarketplaceColumn {
    pub kind: MarketplaceKind,
    pub label: &'static str,
    /// Short ASCII key used for `data-mp` attribute (CSS targeting).
    pub mp_key: &'static str,
    /// Inline SVG markup for the marketplace logo. Embedded directly into HTML.
    pub logo_svg: &'static str,
    /// Brand colour used for the column accent strip and active states.
    pub brand_color: &'static str,
    /// Logo display height override in px (default 34). Use to balance visual weight.
    pub logo_height_px: u32,
}

/// Category of the navigated entity — used as a badge in detailed view and
/// as a technical column in the table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EntityType {
    /// Domain aggregate (a0xx — CRUD entity with UI form).
    Aggregate,
    /// Read-only projection / analytics (p9xx).
    Projection,
    /// Cross-aggregate use case / import (u5xx).
    UseCase,
}

/// Visibility scope of a link across marketplace columns.
#[derive(Clone, Copy, Debug)]
pub enum LinkScope {
    /// Link belongs to every marketplace column.
    All,
    /// Link belongs only to the listed marketplaces.
    Only(&'static [MarketplaceKind]),
}

impl LinkScope {
    pub fn includes(&self, kind: MarketplaceKind) -> bool {
        match self {
            LinkScope::All => true,
            LinkScope::Only(list) => list.contains(&kind),
        }
    }
}

/// One curated link inside a navigator block.
#[derive(Clone, Copy)]
pub struct NavLink {
    /// Tab key passed to `AppGlobalContext::open_tab`.
    pub tab_key: &'static str,
    pub label: &'static str,
    /// Short annotation shown in detailed view and tooltips elsewhere.
    pub annotation: &'static str,
    /// Icon name resolved through `crate::shared::icons::icon`.
    pub icon: &'static str,
    /// Optional access scope. If set and the user lacks read access, the link
    /// is hidden from brief/detailed views and shown as dash in the table.
    pub scope_id: Option<&'static str>,
    pub marketplaces: LinkScope,
    /// Technical category of the entity this link leads to.
    pub entity_type: EntityType,
}

impl EntityType {
    pub fn label(self) -> &'static str {
        match self {
            EntityType::Aggregate => "Агрегат",
            EntityType::Projection => "Проекция",
            EntityType::UseCase => "Сценарий",
        }
    }

    /// CSS modifier class suffix for badge colour.
    pub fn css_mod(self) -> &'static str {
        match self {
            EntityType::Aggregate => "aggregate",
            EntityType::Projection => "projection",
            EntityType::UseCase => "usecase",
        }
    }
}

/// A horizontal section grouping several links by topic
/// (Orders, Sales, Finance, etc.).
#[derive(Clone, Copy)]
pub struct NavBlock {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub links: &'static [NavLink],
}

/// Render mode of a navigator page.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewMode {
    Brief,
    Detailed,
    Table,
}

impl ViewMode {
    pub const ALL: [ViewMode; 3] = [ViewMode::Brief, ViewMode::Detailed, ViewMode::Table];

    pub fn label(self) -> &'static str {
        match self {
            ViewMode::Brief => "Кратко",
            ViewMode::Detailed => "Подробно",
            ViewMode::Table => "Таблица",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            ViewMode::Brief => "brief",
            ViewMode::Detailed => "detailed",
            ViewMode::Table => "table",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            ViewMode::Brief => "list",
            ViewMode::Detailed => "layout-dashboard",
            ViewMode::Table => "columns",
        }
    }
}

impl Default for ViewMode {
    fn default() -> Self {
        ViewMode::Brief
    }
}

//! UI-group access policy registry.
//!
//! Programmatic registry of `nav_id` → role restrictions for specific
//! `CardAnimated` groups that should not be visible to all roles.
//!
//! The registry mirrors the layered structure:
//!   backend/src/system/access ↔ frontend/src/system/access

/// Access restriction for a specific UI card group.
pub struct UiGroupPolicy {
    /// Stable card identifier — `nav_id` prop of `CardAnimated`.
    pub nav_id: &'static str,
    /// Human-readable card label shown in the info panel.
    pub label: &'static str,
    /// Primary role codes that are allowed to see this card.
    /// `admin` (`is_admin=true`) always bypasses this regardless of this list.
    /// Empty slice = no restriction (everyone may see the card).
    pub allowed_roles: &'static [&'static str],
    /// One-sentence explanation shown in the info panel.
    pub reason: &'static str,
}

/// Full registry of UI group access policies.
///
/// Add an entry here whenever a card group needs role-based visibility.
/// `nav_id` values correspond to the `nav_id` prop of `CardAnimated`.
pub const UI_GROUP_POLICIES: &[UiGroupPolicy] = &[
    // ── Nomenclature details ──────────────────────────────────────────────────
    UiGroupPolicy {
        nav_id: "a004_nomenclature_details_production_main",
        label: "Производство (основное)",
        allowed_roles: &["admin", "manager"],
        reason: "Производственные данные доступны только Руководителям и Администраторам",
    },
    // Add more policies below following the same pattern, e.g.:
    // UiGroupPolicy {
    //     nav_id:        "aXXX_..._tab_card",
    //     label:         "...",
    //     allowed_roles: &["admin", "manager"],
    //     reason:        "...",
    // },
];

/// Returns the policy for the given `nav_id`, or `None` if unrestricted.
pub fn find_ui_policy(nav_id: &str) -> Option<&'static UiGroupPolicy> {
    UI_GROUP_POLICIES.iter().find(|p| p.nav_id == nav_id)
}

/// Returns `true` if the user may see the card with this `nav_id`.
///
/// - Always `true` when there is no registered policy.
/// - Always `true` for `is_admin = true` users.
pub fn ui_access_allowed(nav_id: &str, primary_role: &str, is_admin: bool) -> bool {
    if is_admin {
        return true;
    }
    match find_ui_policy(nav_id) {
        None => true,
        Some(p) => p.allowed_roles.contains(&primary_role),
    }
}

/// Human-readable label for a primary role code.
pub fn role_label(code: &str) -> &'static str {
    match code {
        "admin" => "Администратор",
        "manager" => "Руководитель",
        "operator" => "Оператор",
        "viewer" => "Наблюдатель",
        _ => "Неизвестная роль",
    }
}

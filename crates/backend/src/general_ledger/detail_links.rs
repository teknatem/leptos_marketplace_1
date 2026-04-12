#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlDetailLinkKind {
    ProjectionLinked,
    ExternalLinked,
}

#[derive(Debug, Clone, Copy)]
pub struct GlDetailJoinVariant {
    pub join_sql: &'static str,
    pub extra_where_sql: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct GlDetailLinkDescriptor {
    pub resource_table: &'static str,
    pub detail_table: &'static str,
    pub detail_alias: &'static str,
    pub kind: GlDetailLinkKind,
    pub join_variants: &'static [GlDetailJoinVariant],
    pub where_sql: &'static str,
    pub nomenclature_ref_expr: &'static str,
}

const DETAIL_LINKS: &[GlDetailLinkDescriptor] = &[
    GlDetailLinkDescriptor {
        resource_table: "p903_wb_finance_report",
        detail_table: "p903_wb_finance_report",
        detail_alias: "d",
        kind: GlDetailLinkKind::ExternalLinked,
        join_variants: &[
            GlDetailJoinVariant {
                join_sql: "INNER JOIN p903_wb_finance_report d ON d.id = gl_row.registrator_ref",
                extra_where_sql: "gl_row.registrator_ref NOT LIKE 'p903:%'",
            },
            GlDetailJoinVariant {
                join_sql: "INNER JOIN p903_wb_finance_report d ON d.source_row_ref = gl_row.registrator_ref",
                extra_where_sql: "gl_row.registrator_ref LIKE 'p903:%' AND gl_row.registrator_ref NOT LIKE 'p903:%:%'",
            },
            GlDetailJoinVariant {
                join_sql: "INNER JOIN p903_wb_finance_report d ON d.rr_dt = SUBSTR(gl_row.registrator_ref, 6, 10) AND d.rrd_id = CAST(SUBSTR(gl_row.registrator_ref, 17) AS INTEGER)",
                extra_where_sql: "gl_row.registrator_ref LIKE 'p903:%:%'",
            },
        ],
        where_sql: "gl_row.resource_table = 'p903_wb_finance_report' AND gl_row.registrator_type = 'p903_wb_finance_report'",
        nomenclature_ref_expr: "d.a004_nomenclature_ref",
    },
    GlDetailLinkDescriptor {
        resource_table: "p909_mp_order_line_turnovers",
        detail_table: "p909_mp_order_line_turnovers",
        detail_alias: "d",
        kind: GlDetailLinkKind::ProjectionLinked,
        join_variants: &[GlDetailJoinVariant {
            join_sql: "INNER JOIN p909_mp_order_line_turnovers d ON d.general_ledger_ref = gl.id",
            extra_where_sql: "",
        }],
        where_sql: "gl.resource_table = 'p909_mp_order_line_turnovers'",
        nomenclature_ref_expr: "d.nomenclature_ref",
    },
    GlDetailLinkDescriptor {
        resource_table: "p910_mp_unlinked_turnovers",
        detail_table: "p910_mp_unlinked_turnovers",
        detail_alias: "d",
        kind: GlDetailLinkKind::ProjectionLinked,
        join_variants: &[GlDetailJoinVariant {
            join_sql: "INNER JOIN p910_mp_unlinked_turnovers d ON d.general_ledger_ref = gl.id",
            extra_where_sql: "",
        }],
        where_sql: "gl.resource_table = 'p910_mp_unlinked_turnovers'",
        nomenclature_ref_expr: "d.nomenclature_ref",
    },
    GlDetailLinkDescriptor {
        resource_table: "p911_wb_advert_by_items",
        detail_table: "p911_wb_advert_by_items",
        detail_alias: "d",
        kind: GlDetailLinkKind::ProjectionLinked,
        join_variants: &[GlDetailJoinVariant {
            join_sql: "INNER JOIN p911_wb_advert_by_items d ON d.general_ledger_ref = gl.id",
            extra_where_sql: "",
        }],
        where_sql: "gl.resource_table = 'p911_wb_advert_by_items'",
        nomenclature_ref_expr: "d.nomenclature_ref",
    },
];

pub fn descriptor_for_resource_table(
    resource_table: &str,
) -> Option<&'static GlDetailLinkDescriptor> {
    DETAIL_LINKS
        .iter()
        .find(|descriptor| descriptor.resource_table == resource_table)
}

pub fn is_supported_resource_table(resource_table: &str) -> bool {
    descriptor_for_resource_table(resource_table).is_some()
}

#[cfg(test)]
mod tests {
    use super::descriptor_for_resource_table;

    #[test]
    fn p903_descriptor_supports_legacy_registrator_ref_formats() {
        let descriptor = descriptor_for_resource_table("p903_wb_finance_report").unwrap();
        assert_eq!(descriptor.join_variants.len(), 3);
        assert!(descriptor.join_variants[0]
            .join_sql
            .contains("d.id = gl_row.registrator_ref"));
        assert!(descriptor.join_variants[1]
            .join_sql
            .contains("d.source_row_ref = gl_row.registrator_ref"));
        assert!(descriptor.join_variants[2]
            .join_sql
            .contains("d.rr_dt = SUBSTR(gl_row.registrator_ref, 6, 10)"));
        assert!(descriptor.join_variants[2]
            .join_sql
            .contains("d.rrd_id = CAST(SUBSTR(gl_row.registrator_ref, 17) AS INTEGER)"));
        assert_eq!(
            descriptor.join_variants[2].extra_where_sql,
            "gl_row.registrator_ref LIKE 'p903:%:%'"
        );
    }
}

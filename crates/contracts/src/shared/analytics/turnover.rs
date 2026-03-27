use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnoverScope {
    OrderLine,
    Nomenclature,
    Unlinked,
    Both,
}

impl TurnoverScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OrderLine => "order_line",
            Self::Nomenclature => "nomenclature",
            Self::Unlinked => "unlinked",
            Self::Both => "both",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueKind {
    Money,
    Quantity,
    Percent,
    Coefficient,
}

impl ValueKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Money => "money",
            Self::Quantity => "quantity",
            Self::Percent => "percent",
            Self::Coefficient => "coefficient",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "money" => Some(Self::Money),
            "quantity" => Some(Self::Quantity),
            "percent" => Some(Self::Percent),
            "coefficient" => Some(Self::Coefficient),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggKind {
    Sum,
    Avg,
    Last,
    None,
}

impl AggKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sum => "sum",
            Self::Avg => "avg",
            Self::Last => "last",
            Self::None => "none",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "sum" => Some(Self::Sum),
            "avg" => Some(Self::Avg),
            "last" => Some(Self::Last),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionRule {
    PreferFact,
    PreferOper,
    FactOnly,
    OperOnly,
    SumBoth,
}

impl SelectionRule {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PreferFact => "prefer_fact",
            Self::PreferOper => "prefer_oper",
            Self::FactOnly => "fact_only",
            Self::OperOnly => "oper_only",
            Self::SumBoth => "sum_both",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "prefer_fact" => Some(Self::PreferFact),
            "prefer_oper" => Some(Self::PreferOper),
            "fact_only" => Some(Self::FactOnly),
            "oper_only" => Some(Self::OperOnly),
            "sum_both" => Some(Self::SumBoth),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignPolicy {
    Natural,
    IncomePositive,
    ExpensePositive,
}

impl SignPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Natural => "natural",
            Self::IncomePositive => "income_positive",
            Self::ExpensePositive => "expense_positive",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportGroup {
    Revenue,
    Returns,
    Payout,
    Commission,
    Acquiring,
    Logistics,
    Storage,
    Penalty,
    Advertising,
    Cost,
    Quantity,
    Ratio,
    Adjustment,
    Other,
}

impl ReportGroup {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Revenue => "revenue",
            Self::Returns => "returns",
            Self::Payout => "payout",
            Self::Commission => "commission",
            Self::Acquiring => "acquiring",
            Self::Logistics => "logistics",
            Self::Storage => "storage",
            Self::Penalty => "penalty",
            Self::Advertising => "advertising",
            Self::Cost => "cost",
            Self::Quantity => "quantity",
            Self::Ratio => "ratio",
            Self::Adjustment => "adjustment",
            Self::Other => "other",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "revenue" => Some(Self::Revenue),
            "returns" => Some(Self::Returns),
            "payout" => Some(Self::Payout),
            "commission" => Some(Self::Commission),
            "acquiring" => Some(Self::Acquiring),
            "logistics" => Some(Self::Logistics),
            "storage" => Some(Self::Storage),
            "penalty" => Some(Self::Penalty),
            "advertising" => Some(Self::Advertising),
            "cost" => Some(Self::Cost),
            "quantity" => Some(Self::Quantity),
            "ratio" => Some(Self::Ratio),
            "adjustment" => Some(Self::Adjustment),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetProjection {
    P909,
    P910,
}

impl TargetProjection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::P909 => "p909",
            Self::P910 => "p910",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmountColumn {
    Plan,
    Oper,
    Fact,
}

impl AmountColumn {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Oper => "oper",
            Self::Fact => "fact",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnoverLayer {
    Plan,
    Oper,
    Fact,
}

impl TurnoverLayer {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Oper => "oper",
            Self::Fact => "fact",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "plan" => Some(Self::Plan),
            "oper" => Some(Self::Oper),
            "fact" => Some(Self::Fact),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Ordered,
    Sold,
    Returned,
    Fee,
    Adjustment,
    Other,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ordered => "ordered",
            Self::Sold => "sold",
            Self::Returned => "returned",
            Self::Fee => "fee",
            Self::Adjustment => "adjustment",
            Self::Other => "other",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "ordered" => Some(Self::Ordered),
            "sold" => Some(Self::Sold),
            "returned" => Some(Self::Returned),
            "fee" => Some(Self::Fee),
            "adjustment" => Some(Self::Adjustment),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DateSource {
    OrderDate,
    SaleDate,
    FinanceDate,
    RawRowDate,
}

impl DateSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OrderDate => "order_date",
            Self::SaleDate => "sale_date",
            Self::FinanceDate => "finance_date",
            Self::RawRowDate => "raw_row_date",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeySource {
    Srid,
    DocumentNo,
    SaleId,
    CompositeFinanceKey,
    None,
}

impl KeySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Srid => "srid",
            Self::DocumentNo => "document_no",
            Self::SaleId => "sale_id",
            Self::CompositeFinanceKey => "composite_finance_key",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRefStrategy {
    AggregateId,
    CompositeFinanceKey,
    RawRowKey,
}

impl SourceRefStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AggregateId => "aggregate_id",
            Self::CompositeFinanceKey => "composite_finance_key",
            Self::RawRowKey => "raw_row_key",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TurnoverClassDef {
    pub code: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub llm_description: &'static str,
    pub scope: TurnoverScope,
    pub value_kind: ValueKind,
    pub agg_kind: AggKind,
    pub selection_rule: SelectionRule,
    pub sign_policy: SignPolicy,
    pub report_group: ReportGroup,
    pub aliases: &'static [&'static str],
    pub source_examples: &'static [&'static str],
    pub formula_hint: &'static str,
    pub notes: &'static str,
    /// Дебетуемый счёт при создании записи журнала операций.
    /// Пустая строка означает, что оборот не формирует запись журнала.
    pub debit_account: &'static str,
    /// Кредитуемый счёт при создании записи журнала операций.
    pub credit_account: &'static str,
    /// Признак: формирует ли этот оборот запись в sys_general_ledger.
    pub generates_journal_entry: bool,
    /// Человекочитаемый комментарий для журнала операций:
    /// смысл оборота, формула расчёта, счета Дт/Кт.
    pub journal_comment: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct TurnoverMappingRule {
    pub marketplace_code: &'static str,
    pub source_entity: &'static str,
    pub source_variant: &'static str,
    pub target_projection: TargetProjection,
    pub turnover_code: &'static str,
    pub amount_column: AmountColumn,
    pub event_kind: EventKind,
    pub business_date_source: DateSource,
    pub order_key_source: KeySource,
    pub line_key_source: KeySource,
    pub source_ref_strategy: SourceRefStrategy,
    pub match_description: &'static str,
    pub notes: &'static str,
    pub priority: u16,
}

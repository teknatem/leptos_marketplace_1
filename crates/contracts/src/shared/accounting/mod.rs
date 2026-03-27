use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Active,
    Passive,
    ActivePassive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalBalance {
    Debit,
    Credit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatementSection {
    BalanceSheet,
    ProfitLoss,
    OffBalance,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountDef {
    pub code: &'static str,
    pub name: &'static str,
    pub account_type: AccountType,
    pub normal_balance: NormalBalance,
    pub parent_code: Option<&'static str>,
    pub section: StatementSection,
    pub description: &'static str,
}

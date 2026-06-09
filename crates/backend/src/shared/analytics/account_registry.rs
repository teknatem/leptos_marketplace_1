use contracts::shared::accounting::{AccountDef, AccountType, NormalBalance, StatementSection};

pub const ACCOUNT_REGISTRY: &[AccountDef] = &[
    AccountDef {
        code: "62",
        name: "Расчёты с покупателями",
        account_type: AccountType::ActivePassive,
        normal_balance: NormalBalance::Debit,
        parent_code: None,
        section: StatementSection::BalanceSheet,
        description: "Расчёты с покупателями и заказчиками — группа.",
    },
    AccountDef {
        code: "44",
        name: "Расходы на продажу",
        account_type: AccountType::Active,
        normal_balance: NormalBalance::Debit,
        parent_code: None,
        section: StatementSection::ProfitLoss,
        description: "Расходы на продажу — группа.",
    },
    AccountDef {
        code: "4401",
        name: "Расходы на продажу — маркетплейс",
        account_type: AccountType::Active,
        normal_balance: NormalBalance::Debit,
        parent_code: Some("44"),
        section: StatementSection::ProfitLoss,
        description: "Комиссии, логистика, хранение и прочие удержания маркетплейса.",
    },
    AccountDef {
        code: "41",
        name: "Товары",
        account_type: AccountType::Active,
        normal_balance: NormalBalance::Debit,
        parent_code: None,
        section: StatementSection::BalanceSheet,
        description: "Товары на складе и в пути.",
    },
    AccountDef {
        code: "90",
        name: "Продажи",
        account_type: AccountType::ActivePassive,
        normal_balance: NormalBalance::Credit,
        parent_code: None,
        section: StatementSection::ProfitLoss,
        description: "Счёт продаж — группа.",
    },
    AccountDef {
        code: "9001",
        name: "Выручка от продаж",
        account_type: AccountType::Passive,
        normal_balance: NormalBalance::Credit,
        parent_code: Some("90"),
        section: StatementSection::ProfitLoss,
        description: "Выручка от реализации товаров через маркетплейсы.",
    },
    AccountDef {
        code: "9002",
        name: "Себестоимость продаж",
        account_type: AccountType::Active,
        normal_balance: NormalBalance::Debit,
        parent_code: Some("90"),
        section: StatementSection::ProfitLoss,
        description: "Себестоимость реализованных товаров.",
    },
    AccountDef {
        code: "91",
        name: "Прочие доходы и расходы",
        account_type: AccountType::ActivePassive,
        normal_balance: NormalBalance::Credit,
        parent_code: None,
        section: StatementSection::ProfitLoss,
        description: "Прочие доходы — соинвестирование маркетплейса, бонусы и иные внереализационные поступления.",
    },
    AccountDef {
        code: "76",
        name: "Расчёты с прочими дебиторами и кредиторами",
        account_type: AccountType::ActivePassive,
        normal_balance: NormalBalance::Debit,
        parent_code: None,
        section: StatementSection::BalanceSheet,
        description: "Расчёты с прочими контрагентами — группа.",
    },
    AccountDef {
        code: "7609",
        name: "Расчёты с маркетплейсом",
        account_type: AccountType::ActivePassive,
        normal_balance: NormalBalance::Debit,
        parent_code: Some("76"),
        section: StatementSection::BalanceSheet,
        description: "Взаиморасчёты с маркетплейсами (комиссии, удержания, выплаты). \
            Дебетовое сальдо контура маркетплейса = сумма, доступная к перечислению поставщику.",
    },
    AccountDef {
        code: "76YB",
        name: "Баланс баллов/промо (Яндекс.Маркет)",
        account_type: AccountType::ActivePassive,
        normal_balance: NormalBalance::Debit,
        parent_code: Some("76"),
        section: StatementSection::BalanceSheet,
        description: "Информационный кошелёк баллов/промо маркетплейса (операции payment_status «Справочно: …баланс»). \
            Ведётся отдельно от денежных расчётов 7609 — не является живыми деньгами к выплате.",
    },
    AccountDef {
        code: "76YA",
        name: "Деньги покупателей у Я.Маркет (предоплаты в пути)",
        account_type: AccountType::ActivePassive,
        normal_balance: NormalBalance::Debit,
        parent_code: Some("76"),
        section: StatementSection::BalanceSheet,
        description: "Деньги покупателей, удерживаемые маркетплейсом по оплаченным, но ещё не \
            отгруженным заказам. На отгрузке переносится на 7609 (становится доступно к перечислению \
            поставщику). Дебетовое сальдо = предоплаты в пути; зеркалит открытое Кт-сальдо 62.",
    },
    AccountDef {
        code: "51",
        name: "Расчётный счёт",
        account_type: AccountType::Active,
        normal_balance: NormalBalance::Debit,
        parent_code: None,
        section: StatementSection::BalanceSheet,
        description: "Денежные средства на расчётном счёте в банке. Дебетуется при перечислении \
            маркетплейсом выплаты поставщику.",
    },
];

pub fn get_account(code: &str) -> Option<&'static AccountDef> {
    ACCOUNT_REGISTRY.iter().find(|a| a.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_codes_are_unique() {
        let mut codes = std::collections::HashSet::new();
        for account in ACCOUNT_REGISTRY {
            assert!(
                codes.insert(account.code),
                "duplicate account code: {}",
                account.code
            );
        }
    }

    #[test]
    fn parent_codes_exist() {
        for account in ACCOUNT_REGISTRY {
            if let Some(parent) = account.parent_code {
                assert!(
                    get_account(parent).is_some(),
                    "account {} references unknown parent {}",
                    account.code,
                    parent
                );
            }
        }
    }
}

use super::progress_tracker::ProgressTracker;
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use contracts::usecases::u508_repost_documents::{
    aggregate::AggregateOption,
    aggregate_request::AggregateRepostRequest,
    progress::RepostStatus,
    projection::ProjectionOption,
    request::RepostRequest,
    response::{RepostResponse, RepostStartStatus},
};
use std::sync::Arc;
use uuid::Uuid;

const P904_SALES_DATA: &str = "p904_sales_data";
const P903_FINANCE_REPORT: &str = "p903_wb_finance_report";
const A012_WB_SALES: &str = "a012_wb_sales";
const A026_WB_ADVERT_DAILY: &str = "a026_wb_advert_daily";

pub struct RepostExecutor {
    pub progress_tracker: Arc<ProgressTracker>,
}

impl RepostExecutor {
    pub fn new(progress_tracker: Arc<ProgressTracker>) -> Self {
        Self { progress_tracker }
    }

    pub fn list_available_projections(&self) -> Vec<ProjectionOption> {
        vec![
            ProjectionOption {
                key: P903_FINANCE_REPORT.to_string(),
                label: "WB Finance Report".to_string(),
                description:
                    "Р›РѕРєР°Р»СЊРЅР°СЏ РїРµСЂРµСЃР±РѕСЂРєР° general ledger РїРѕ СЃРѕС…СЂР°РЅРµРЅРЅС‹Рј СЃС‚СЂРѕРєР°Рј p903_wb_finance_report".to_string(),
            },
            ProjectionOption {
                key: P904_SALES_DATA.to_string(),
                label: "Sales Data".to_string(),
                description:
                    "РџРµСЂРµРїСЂРѕРІРµРґРµРЅРёРµ РґРѕРєСѓРјРµРЅС‚РѕРІ РїРѕ registrator_ref РёР· p904_sales_data".to_string(),
            },
        ]
    }

    pub fn list_available_aggregates(&self) -> Vec<AggregateOption> {
        vec![
            AggregateOption {
                key: A012_WB_SALES.to_string(),
                label: "WB Sales".to_string(),
                description:
                    "РџРµСЂРµРїСЂРѕРІРµРґРµРЅРёРµ РґРѕРєСѓРјРµРЅС‚РѕРІ a012_wb_sales СЃ РїРµСЂРµСЃР±РѕСЂРєРѕР№ СЃРІСЏР·Р°РЅРЅС‹С… РїСЂРѕРµРєС†РёР№"
                        .to_string(),
            },
            AggregateOption {
            key: A026_WB_ADVERT_DAILY.to_string(),
            label: "WB Advert Daily".to_string(),
            description: "РџРµСЂРµРїСЂРѕРІРµРґРµРЅРёРµ РїСЂРѕРІРµРґРµРЅРЅС‹С… РґРѕРєСѓРјРµРЅС‚РѕРІ a026_wb_advert_daily СЃ РїРµСЂРµСЃР±РѕСЂРєРѕР№ СЃРІСЏР·Р°РЅРЅС‹С… РїСЂРѕРµРєС†РёР№".to_string(),
            },
        ]
    }

    pub async fn start_repost(&self, request: RepostRequest) -> Result<RepostResponse> {
        Self::validate_request(&request)?;

        let session_id = Uuid::new_v4().to_string();
        self.progress_tracker.create_session(session_id.clone());

        let executor = Arc::new(Self {
            progress_tracker: self.progress_tracker.clone(),
        });
        let sid = session_id.clone();
        let req = request.clone();

        tokio::spawn(async move {
            if let Err(error) = executor.execute_repost(&sid, &req).await {
                tracing::error!("Projection repost failed: {}", error);
                executor
                    .progress_tracker
                    .add_error(&sid, format!("Repost failed: {}", error));
                executor
                    .progress_tracker
                    .complete_session(&sid, RepostStatus::Failed);
            }
        });

        Ok(RepostResponse {
            session_id,
            status: RepostStartStatus::Started,
            message: "Repost started".to_string(),
        })
    }

    pub async fn start_aggregate_repost(
        &self,
        request: AggregateRepostRequest,
    ) -> Result<RepostResponse> {
        Self::validate_aggregate_request(&request)?;

        let session_id = Uuid::new_v4().to_string();
        self.progress_tracker.create_session(session_id.clone());

        let executor = Arc::new(Self {
            progress_tracker: self.progress_tracker.clone(),
        });
        let sid = session_id.clone();
        let req = request.clone();

        tokio::spawn(async move {
            if let Err(error) = executor.execute_aggregate_repost(&sid, &req).await {
                tracing::error!("Aggregate repost failed: {}", error);
                executor
                    .progress_tracker
                    .add_error(&sid, format!("Repost failed: {}", error));
                executor
                    .progress_tracker
                    .complete_session(&sid, RepostStatus::Failed);
            }
        });

        Ok(RepostResponse {
            session_id,
            status: RepostStartStatus::Started,
            message: "Aggregate repost started".to_string(),
        })
    }

    pub fn get_progress(
        &self,
        session_id: &str,
    ) -> Option<contracts::usecases::u508_repost_documents::progress::RepostProgress> {
        self.progress_tracker.get_progress(session_id)
    }

    fn validate_request(request: &RepostRequest) -> Result<()> {
        if request.projection_key != P903_FINANCE_REPORT
            && request.projection_key != P904_SALES_DATA
        {
            return Err(anyhow!(
                "Unsupported projection_key: {}",
                request.projection_key
            ));
        }

        let date_from = NaiveDate::parse_from_str(&request.date_from, "%Y-%m-%d")
            .map_err(|_| anyhow!("Invalid date_from: {}", request.date_from))?;
        let date_to = NaiveDate::parse_from_str(&request.date_to, "%Y-%m-%d")
            .map_err(|_| anyhow!("Invalid date_to: {}", request.date_to))?;

        if date_from > date_to {
            return Err(anyhow!("date_from must be less than or equal to date_to"));
        }

        Ok(())
    }

    fn validate_aggregate_request(request: &AggregateRepostRequest) -> Result<()> {
        if request.aggregate_key != A012_WB_SALES && request.aggregate_key != A026_WB_ADVERT_DAILY {
            return Err(anyhow!(
                "Unsupported aggregate_key: {}",
                request.aggregate_key
            ));
        }

        let date_from = NaiveDate::parse_from_str(&request.date_from, "%Y-%m-%d")
            .map_err(|_| anyhow!("Invalid date_from: {}", request.date_from))?;
        let date_to = NaiveDate::parse_from_str(&request.date_to, "%Y-%m-%d")
            .map_err(|_| anyhow!("Invalid date_to: {}", request.date_to))?;

        if date_from > date_to {
            return Err(anyhow!("date_from must be less than or equal to date_to"));
        }

        Ok(())
    }

    async fn execute_repost(&self, session_id: &str, request: &RepostRequest) -> Result<()> {
        let registrators = match request.projection_key.as_str() {
            P903_FINANCE_REPORT => {
                self.progress_tracker.set_total(session_id, 1);
                self.progress_tracker.update_progress(
                    session_id,
                    0,
                    0,
                    Some("Rebuilding p903 general ledger".to_string()),
                );
                crate::projections::p903_wb_finance_report::service::rebuild_range_from_existing(
                    &request.date_from,
                    &request.date_to,
                )
                .await?;
                self.progress_tracker.update_progress(
                    session_id,
                    1,
                    1,
                    Some("Rebuilding p903 general ledger".to_string()),
                );
                self.progress_tracker
                    .complete_session(session_id, RepostStatus::Completed);
                return Ok(());
            }
            P904_SALES_DATA => {
                crate::projections::p904_sales_data::repository::list_registrators_by_period(
                    &request.date_from,
                    &request.date_to,
                )
                .await?
            }
            _ => {
                return Err(anyhow!(
                    "Unsupported projection_key: {}",
                    request.projection_key
                ))
            }
        };

        let total = registrators.len() as i32;
        self.progress_tracker.set_total(session_id, total);

        let mut reposted = 0;

        for (index, registrator) in registrators.iter().enumerate() {
            let current_item = format!(
                "{} {}",
                registrator.registrator_type, registrator.registrator_ref
            );
            self.progress_tracker.update_progress(
                session_id,
                index as i32,
                reposted,
                Some(current_item.clone()),
            );

            let registrator_id = match Uuid::parse_str(&registrator.registrator_ref) {
                Ok(value) => value,
                Err(error) => {
                    self.progress_tracker.add_error(
                        session_id,
                        format!(
                            "Invalid registrator_ref {}: {}",
                            registrator.registrator_ref, error
                        ),
                    );
                    self.progress_tracker.update_progress(
                        session_id,
                        (index + 1) as i32,
                        reposted,
                        Some(current_item),
                    );
                    continue;
                }
            };

            if let Err(error) = dispatch_repost(&registrator.registrator_type, registrator_id).await
            {
                self.progress_tracker.add_error(
                    session_id,
                    format!(
                        "Failed to repost {} {}: {}",
                        registrator.registrator_type, registrator.registrator_ref, error
                    ),
                );
                self.progress_tracker.update_progress(
                    session_id,
                    (index + 1) as i32,
                    reposted,
                    Some(current_item),
                );
                continue;
            }

            reposted += 1;
            self.progress_tracker.update_progress(
                session_id,
                (index + 1) as i32,
                reposted,
                Some(current_item),
            );
        }

        self.progress_tracker
            .update_progress(session_id, total, reposted, None);

        let final_status = if self
            .progress_tracker
            .get_progress(session_id)
            .map(|progress| progress.errors > 0)
            .unwrap_or(false)
        {
            RepostStatus::CompletedWithErrors
        } else {
            RepostStatus::Completed
        };

        self.progress_tracker
            .complete_session(session_id, final_status);

        Ok(())
    }

    async fn execute_aggregate_repost(
        &self,
        session_id: &str,
        request: &AggregateRepostRequest,
    ) -> Result<()> {
        let document_ids = match request.aggregate_key.as_str() {
            A012_WB_SALES => {
                crate::domain::a012_wb_sales::repository::list_ids_by_sale_date_range(
                    &request.date_from,
                    &request.date_to,
                    request.only_posted,
                )
                .await?
            }
            A026_WB_ADVERT_DAILY => {
                crate::domain::a026_wb_advert_daily::repository::list_ids_by_period(
                    &request.date_from,
                    &request.date_to,
                    request.only_posted,
                )
                .await?
            }
            _ => {
                return Err(anyhow!(
                    "Unsupported aggregate_key: {}",
                    request.aggregate_key
                ));
            }
        };

        let total = document_ids.len() as i32;
        self.progress_tracker.set_total(session_id, total);

        let mut reposted = 0;

        for (index, document_id) in document_ids.iter().enumerate() {
            let current_item = format!("{} {}", request.aggregate_key, document_id);
            self.progress_tracker.update_progress(
                session_id,
                index as i32,
                reposted,
                Some(current_item.clone()),
            );

            let aggregate_id = match Uuid::parse_str(document_id) {
                Ok(value) => value,
                Err(error) => {
                    self.progress_tracker.add_error(
                        session_id,
                        format!("Invalid aggregate id {}: {}", document_id, error),
                    );
                    self.progress_tracker.update_progress(
                        session_id,
                        (index + 1) as i32,
                        reposted,
                        Some(current_item),
                    );
                    continue;
                }
            };

            if let Err(error) =
                dispatch_aggregate_repost(&request.aggregate_key, aggregate_id).await
            {
                self.progress_tracker.add_error(
                    session_id,
                    format!(
                        "Failed to repost {} {}: {}",
                        request.aggregate_key, document_id, error
                    ),
                );
                self.progress_tracker.update_progress(
                    session_id,
                    (index + 1) as i32,
                    reposted,
                    Some(current_item),
                );
                continue;
            }

            reposted += 1;
            self.progress_tracker.update_progress(
                session_id,
                (index + 1) as i32,
                reposted,
                Some(current_item),
            );
        }

        self.progress_tracker
            .update_progress(session_id, total, reposted, None);

        let final_status = if self
            .progress_tracker
            .get_progress(session_id)
            .map(|progress| progress.errors > 0)
            .unwrap_or(false)
        {
            RepostStatus::CompletedWithErrors
        } else {
            RepostStatus::Completed
        };

        self.progress_tracker
            .complete_session(session_id, final_status);

        Ok(())
    }
}

async fn dispatch_repost(registrator_type: &str, registrator_id: Uuid) -> Result<()> {
    match registrator_type {
        "WB_Sales" => crate::domain::a012_wb_sales::posting::post_document(registrator_id).await,
        "OZON_Transactions" => {
            crate::domain::a014_ozon_transactions::posting::post_document(registrator_id).await
        }
        "YM_Order" => crate::domain::a013_ym_order::posting::post_document(registrator_id).await,
        "YM_Returns" => {
            crate::domain::a016_ym_returns::posting::post_document(registrator_id).await
        }
        "OZON_FBS" => {
            crate::domain::a010_ozon_fbs_posting::posting::post_document(registrator_id).await
        }
        "OZON_FBO" => {
            crate::domain::a011_ozon_fbo_posting::posting::post_document(registrator_id).await
        }
        "OZON_Returns" => {
            crate::domain::a009_ozon_returns::posting::post_document(registrator_id).await
        }
        _ => Err(anyhow!(
            "Unsupported registrator_type: {}",
            registrator_type
        )),
    }
}

async fn dispatch_aggregate_repost(aggregate_key: &str, aggregate_id: Uuid) -> Result<()> {
    match aggregate_key {
        A012_WB_SALES => crate::domain::a012_wb_sales::posting::post_document(aggregate_id).await,
        A026_WB_ADVERT_DAILY => {
            crate::domain::a026_wb_advert_daily::posting::post_document(aggregate_id).await
        }
        _ => Err(anyhow!("Unsupported aggregate_key: {}", aggregate_key)),
    }
}


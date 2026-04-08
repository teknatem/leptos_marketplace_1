use super::progress_tracker::ProgressTracker;
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use contracts::domain::common::AggregateId;
use contracts::usecases::u508_repost_documents::{
    aggregate::AggregateOption,
    aggregate_request::AggregateRepostRequest,
    progress::RepostStatus,
    projection::ProjectionOption,
    request::RepostRequest,
    response::{RepostResponse, RepostStartStatus},
};
use std::sync::{
    atomic::{AtomicI32, Ordering},
    Arc,
};
use uuid::Uuid;

const P904_SALES_DATA: &str = "p904_sales_data";
const P903_FINANCE_REPORT: &str = "p903_wb_finance_report";
const A012_WB_SALES: &str = "a012_wb_sales";
const A021_PRODUCTION_OUTPUT: &str = "a021_production_output";
const A023_PURCHASE_OF_GOODS: &str = "a023_purchase_of_goods";
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
                    "Локальная пересборка general ledger по сохранённым строкам p903_wb_finance_report".to_string(),
            },
            ProjectionOption {
                key: P904_SALES_DATA.to_string(),
                label: "Sales Data".to_string(),
                description:
                    "Перепроведение документов по registrator_ref из p904_sales_data".to_string(),
            },
        ]
    }

    pub fn list_available_aggregates(&self) -> Vec<AggregateOption> {
        vec![
            AggregateOption {
                key: A012_WB_SALES.to_string(),
                label: "WB Sales".to_string(),
                description:
                    "Перепроведение документов a012_wb_sales с пересборкой связанных проекций"
                        .to_string(),
            },
            AggregateOption {
                key: A021_PRODUCTION_OUTPUT.to_string(),
                label: "Production Output".to_string(),
                description:
                    "Перепроведение документов a021_production_output с пересборкой связанных проекций"
                        .to_string(),
            },
            AggregateOption {
                key: A023_PURCHASE_OF_GOODS.to_string(),
                label: "Purchase Of Goods".to_string(),
                description:
                    "Перепроведение документов a023_purchase_of_goods с пересборкой связанных проекций"
                        .to_string(),
            },
            AggregateOption {
                key: A026_WB_ADVERT_DAILY.to_string(),
                label: "WB Advert Daily".to_string(),
                description: "Перепроведение проведённых документов a026_wb_advert_daily с пересборкой связанных проекций".to_string(),
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
        if request.aggregate_key != A012_WB_SALES
            && request.aggregate_key != A021_PRODUCTION_OUTPUT
            && request.aggregate_key != A023_PURCHASE_OF_GOODS
            && request.aggregate_key != A026_WB_ADVERT_DAILY
        {
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
        if request.aggregate_key == A012_WB_SALES {
            return self.execute_a012_chunked_repost(session_id, request).await;
        }

        let document_ids = match request.aggregate_key.as_str() {
            A021_PRODUCTION_OUTPUT => {
                crate::domain::a021_production_output::repository::list_ids_by_document_date_range(
                    &request.date_from,
                    &request.date_to,
                    request.only_posted,
                )
                .await?
            }
            A023_PURCHASE_OF_GOODS => {
                crate::domain::a023_purchase_of_goods::repository::list_ids_by_document_date_range(
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

        // Параллельная обработка с ограничением параллелизма.
        // SQLite (WAL) сериализует запись, но параллелизм ускоряет CPU-bound
        // вычисления и перекрытие read-фазы (get_by_id, lookups) с write-фазой.
        const CONCURRENCY: usize = 4;
        let semaphore = Arc::new(tokio::sync::Semaphore::new(CONCURRENCY));
        let processed = Arc::new(AtomicI32::new(0));
        let reposted = Arc::new(AtomicI32::new(0));
        let tracker = self.progress_tracker.clone();
        let session_id_str = session_id.to_string();
        let aggregate_key = request.aggregate_key.clone();

        let mut join_set = tokio::task::JoinSet::new();

        for document_id_str in document_ids {
            let aggregate_id = match Uuid::parse_str(&document_id_str) {
                Ok(id) => id,
                Err(error) => {
                    tracker.add_error(
                        &session_id_str,
                        format!("Invalid aggregate id {}: {}", document_id_str, error),
                    );
                    processed.fetch_add(1, Ordering::Relaxed);
                    let p = processed.load(Ordering::Relaxed);
                    let r = reposted.load(Ordering::Relaxed);
                    tracker.update_progress(&session_id_str, p, r, None);
                    continue;
                }
            };

            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| anyhow!("Semaphore closed"))?;

            let tracker_task = tracker.clone();
            let sid = session_id_str.clone();
            let agg_key = aggregate_key.clone();
            let processed_ref = processed.clone();
            let reposted_ref = reposted.clone();

            join_set.spawn(async move {
                let _permit = permit;
                let result = dispatch_aggregate_repost(&agg_key, aggregate_id).await;
                let proc_count = processed_ref.fetch_add(1, Ordering::Relaxed) + 1;
                let repo_count = match result {
                    Ok(()) => reposted_ref.fetch_add(1, Ordering::Relaxed) + 1,
                    Err(error) => {
                        tracker_task.add_error(
                            &sid,
                            format!("Failed to repost {} {}: {}", agg_key, aggregate_id, error),
                        );
                        reposted_ref.load(Ordering::Relaxed)
                    }
                };
                tracker_task.update_progress(&sid, proc_count, repo_count, None);
            });
        }

        while let Some(task_result) = join_set.join_next().await {
            task_result.map_err(|e| anyhow!("Task panicked: {}", e))?;
        }

        let final_reposted = reposted.load(Ordering::Relaxed);
        self.progress_tracker
            .update_progress(session_id, total, final_reposted, None);

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

    async fn execute_a012_chunked_repost(
        &self,
        session_id: &str,
        request: &AggregateRepostRequest,
    ) -> Result<()> {
        let connection_labels: std::collections::HashMap<String, String> =
            crate::domain::a006_connection_mp::service::list_all()
                .await?
                .into_iter()
                .map(|connection| {
                    let label = if connection.base.description.trim().is_empty() {
                        connection.base.code.clone()
                    } else {
                        connection.base.description.clone()
                    };
                    (connection.base.id.as_string(), label)
                })
                .collect();

        let chunks =
            crate::domain::a012_wb_sales::repository::list_repost_chunks_by_sale_date_range(
                &request.date_from,
                &request.date_to,
                request.only_posted,
                &request.connection_mp_refs,
            )
            .await?;

        let mut prepared_chunks = Vec::with_capacity(chunks.len());
        let mut total_documents = 0_i32;

        for chunk in chunks {
            let ids =
                crate::domain::a012_wb_sales::repository::list_ids_by_sale_date_and_connection(
                    &chunk.sale_date,
                    &chunk.connection_mp_ref,
                    request.only_posted,
                )
                .await?;
            total_documents += ids.len() as i32;
            prepared_chunks.push((chunk, ids));
        }

        self.progress_tracker.set_total(session_id, total_documents);
        self.progress_tracker
            .set_chunks_total(session_id, prepared_chunks.len() as i32);

        let mut processed = 0_i32;
        let mut reposted = 0_i32;
        let mut chunks_processed = 0_i32;
        let mut day_start = 0_usize;
        while day_start < prepared_chunks.len() {
            let current_day = prepared_chunks[day_start].0.sale_date.clone();
            let mut day_end = day_start;
            while day_end < prepared_chunks.len()
                && prepared_chunks[day_end].0.sale_date == current_day
            {
                day_end += 1;
            }

            let mut posting_cache =
                crate::domain::a012_wb_sales::service::PostingPreparationCache::default();
            let day_document_ids = prepared_chunks[day_start..day_end]
                .iter()
                .flat_map(|(_, ids)| ids.iter().cloned())
                .collect::<Vec<_>>();
            if !day_document_ids.is_empty() {
                let day_documents =
                    crate::domain::a012_wb_sales::repository::list_by_ids(&day_document_ids)
                        .await?;
                crate::domain::a012_wb_sales::service::preload_prod_cost_context_for_documents(
                    &mut posting_cache,
                    &day_documents,
                )
                .await?;
            }

            for (chunk, ids) in &prepared_chunks[day_start..day_end] {
                let cabinet_label = connection_labels
                    .get(&chunk.connection_mp_ref)
                    .cloned()
                    .unwrap_or_else(|| chunk.connection_mp_ref.clone());
                let chunk_label = format!("{} | {}", chunk.sale_date, cabinet_label);
                self.progress_tracker.update_chunk_progress(
                    session_id,
                    chunks_processed,
                    Some(chunk.sale_date.clone()),
                    Some(chunk.connection_mp_ref.clone()),
                    Some(chunk_label.clone()),
                );

                for document_id in ids {
                    let current_item = format!("{} {}", A012_WB_SALES, document_id);
                    self.progress_tracker.update_progress(
                        session_id,
                        processed,
                        reposted,
                        Some(current_item.clone()),
                    );

                    let aggregate_id = match Uuid::parse_str(document_id) {
                        Ok(id) => id,
                        Err(error) => {
                            processed += 1;
                            self.progress_tracker.add_error(
                                session_id,
                                format!("Invalid aggregate id {}: {}", document_id, error),
                            );
                            self.progress_tracker.update_progress(
                                session_id,
                                processed,
                                reposted,
                                Some(current_item),
                            );
                            continue;
                        }
                    };

                    match crate::domain::a012_wb_sales::posting::post_document_with_cache(
                        aggregate_id,
                        &mut posting_cache,
                    )
                    .await
                    {
                        Ok(()) => reposted += 1,
                        Err(error) => self.progress_tracker.add_error(
                            session_id,
                            format!(
                                "Failed to repost {} {}: {}",
                                A012_WB_SALES, aggregate_id, error
                            ),
                        ),
                    }

                    processed += 1;
                    self.progress_tracker.update_progress(
                        session_id,
                        processed,
                        reposted,
                        Some(current_item),
                    );
                }

                chunks_processed += 1;
                self.progress_tracker.update_chunk_progress(
                    session_id,
                    chunks_processed,
                    Some(chunk.sale_date.clone()),
                    Some(chunk.connection_mp_ref.clone()),
                    Some(chunk_label),
                );
            }

            day_start = day_end;
        }

        self.progress_tracker
            .update_progress(session_id, processed, reposted, None);

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
        "a021_production_output" => {
            crate::domain::a021_production_output::service::post_document(registrator_id).await
        }
        "a023_purchase_of_goods" => {
            crate::domain::a023_purchase_of_goods::service::post_document(registrator_id).await
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
        A021_PRODUCTION_OUTPUT => {
            crate::domain::a021_production_output::service::post_document(aggregate_id).await
        }
        A023_PURCHASE_OF_GOODS => {
            crate::domain::a023_purchase_of_goods::service::post_document(aggregate_id).await
        }
        A026_WB_ADVERT_DAILY => {
            crate::domain::a026_wb_advert_daily::posting::post_document(aggregate_id).await
        }
        _ => Err(anyhow!("Unsupported aggregate_key: {}", aggregate_key)),
    }
}

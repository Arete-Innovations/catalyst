use crate::cata_log;
use crate::database::db::establish_connection;
use crate::database::schema::cronjobs::dsl::*;
use crate::structs::*;
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use std::collections::HashMap;
use std::process::Command;
use tokio::time::{self, Duration};

struct ScheduledJob {
    cronjob: Cronjobs,
    interval: i64,
    next_run: i64,
}

pub async fn scheduler() {
    let mut jobs: HashMap<i32, ScheduledJob> = HashMap::new();

    let mut interval = time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;
        cata_log!(CronjobExecution, "Checking scheduled jobs...");

        let mut connection = establish_connection().await;
        update_jobs(&mut connection, &mut jobs).await;

        let current_time = Utc::now().timestamp();
        for job in jobs.values_mut() {
            if current_time >= job.next_run {
                let job_name = job.cronjob.name.clone();
                {
                    let msg = format!("Running job {}", job_name);
                    cata_log!(CronjobExecution, msg);
                }

                let job_name_clone = job_name.clone();
                tokio::spawn(async move {
                    if let Err(err) = run_cronjob(&job_name_clone).await {
                        let msg = format!("Failed to run job {}: {}", job_name_clone, err);
                        cata_log!(CronjobError, msg);
                    }
                });

                job.next_run = current_time + job.interval;

                {
                    let msg_prefix = format!("Failed to update last run for job {}: ", job.cronjob.id);
                    if let Err(err) = update_last_run(&mut connection, job.cronjob.id).await {
                        let full_msg = format!("{}{}", msg_prefix, err);
                        cata_log!(CronjobError, full_msg);
                    }
                }
            }
        }
    }
}

async fn update_jobs(conn: &mut AsyncPgConnection, jobs: &mut HashMap<i32, ScheduledJob>) {
    match cronjobs.load::<Cronjobs>(conn).await {
        Ok(cronjob_list) => {
            for cronjob in cronjob_list {
                let interval = cronjob.timer as i64;
                let next_run = Utc::now().timestamp() + interval;
                jobs.entry(cronjob.id).or_insert(ScheduledJob { cronjob, interval, next_run });
            }
            {
                let msg = format!("Loaded {} cronjobs", jobs.len());
                cata_log!(CronjobExecution, msg);
            }
        }
        Err(err) => {
            let msg = format!("Failed to load cronjobs: {}", err);
            cata_log!(CronjobError, msg);
        }
    }
}

async fn run_cronjob(job_name: &str) -> Result<(), String> {
    let exec_status = Command::new(format!("target/release/{}", job_name)).status().map_err(|e| format!("Failed to execute job: {}", e))?;

    if exec_status.success() {
        let msg = format!("Job {} executed successfully", job_name);
        cata_log!(CronjobExecution, msg);
        Ok(())
    } else {
        Err(format!("Job {} failed with status {:?}", job_name, exec_status))
    }
}

async fn update_last_run(conn: &mut AsyncPgConnection, job_id: i32) -> Result<(), diesel::result::Error> {
    let current_time = Utc::now().timestamp();

    match diesel::update(cronjobs.find(job_id)).set((last_run.eq(current_time), status.eq("completed"))).execute(conn).await {
        Ok(_) => Ok(()),
        Err(err) => {
            let msg = format!("Failed to update last run for job {}: {}", job_id, err);
            cata_log!(CronjobError, msg);
            Err(err)
        }
    }
}


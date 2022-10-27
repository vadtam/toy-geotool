use std::env;
use sqlx::{PgPool, Acquire, PgConnection, Postgres, Transaction, Pool};
use sqlx::postgres::PgPoolOptions;
use std::{thread, time};

mod well;
use well::*;
mod point;
use point::*;
mod tag;
use tag::*;
mod lttb;
use lttb::*;

async fn get_computable_time_to(pool: &Pool<Postgres>, well: &Well) -> i32 {
    let mut time_to: i32 = 0;
    if well.bhp_mode == ComputationMode::Client {
        let point_maybe = get_last_point_f32_pool(pool, well, 1).await;
        if point_maybe.is_some() {
            let point = point_maybe.unwrap();
            if time_to == 0 {
                time_to = point.time;
            } else {
                if point.time < time_to {
                    time_to = point.time;
                }
            }
        } else {
            return 0;  // sic! to ensure all 'Client-mode' tags are non-empty
        }
    }
    if well.bht_mode == ComputationMode::Client {
        let point_maybe = get_last_point_f32_pool(pool, well, 2).await;
        if point_maybe.is_some() {
            let point = point_maybe.unwrap();
            if time_to == 0 {
                time_to = point.time;
            } else {
                if point.time < time_to {
                    time_to = point.time;
                }
            }
        } else {
            return 0;  // sic! to ensure all 'Client-mode' tags are non-empty
        }
    }
    if well.whp_mode == ComputationMode::Client {
        let point_maybe = get_last_point_f32_pool(pool, well, 3).await;
        if point_maybe.is_some() {
            let point = point_maybe.unwrap();
            if time_to == 0 {
                time_to = point.time;
            } else {
                if point.time < time_to {
                    time_to = point.time;
                }
            }
        } else {
            return 0;  // sic! to ensure all 'Client-mode' tags are non-empty
        }
    }
    if well.rate_mode == ComputationMode::Client {
        let point_maybe = get_last_point_f32_pool(pool, well, 4).await;
        if point_maybe.is_some() {
            let point = point_maybe.unwrap();
            if time_to == 0 {
                time_to = point.time;
            } else {
                if point.time < time_to {
                    time_to = point.time;
                }
            }
        } else {
            return 0;  // sic! to ensure all 'Client-mode' tags are non-empty
        }
    }
    if well.rho_mode == ComputationMode::Client {
        let point_maybe = get_last_point_f32_pool(pool, well, 5).await;
        if point_maybe.is_some() {
            let point = point_maybe.unwrap();
            if time_to == 0 {
                time_to = point.time;
            } else {
                if point.time < time_to {
                    time_to = point.time;
                }
            }
        } else {
            return 0;  // sic! to ensure all 'Client-mode' tags are non-empty
        }
    }
    if well.vtot_mode == ComputationMode::Client {
        let point_maybe = get_last_point_f64_pool(pool, well, 6).await;
        if point_maybe.is_some() {
            let point = point_maybe.unwrap();
            if time_to == 0 {
                time_to = point.time;
            } else {
                if point.time < time_to {
                    time_to = point.time;
                }
            }
        } else {
            return 0;  // sic! to ensure all 'Client-mode' tags are non-empty
        }
    }
    if well.ii_mode == ComputationMode::Client {
        let point_maybe = get_last_point_f32_pool(pool, well, 7).await;
        if point_maybe.is_some() {
            let point = point_maybe.unwrap();
            if time_to == 0 {
                time_to = point.time;
            } else {
                if point.time < time_to {
                    time_to = point.time;
                }
            }
        } else {
            return 0;  // sic! to ensure all 'Client-mode' tags are non-empty
        }
    }
    time_to
}

async fn get_optimised_from_time(pool: &Pool<Postgres>, well: &Well) -> i32 {
    if well.computed_to > 0 {
        well.computed_to
    } else {
        let mut from_time = 0;
        if well.bhp_mode == ComputationMode::Client {
            let point_maybe = get_first_point_f32_pool(pool, well, 1).await;
            if point_maybe.is_some() {
                let point = point_maybe.unwrap();
                if point.time > from_time {
                    from_time = point.time;
                }
            }
        }
        if well.bht_mode == ComputationMode::Client {
            let point_maybe = get_first_point_f32_pool(pool, well, 2).await;
            if point_maybe.is_some() {
                let point = point_maybe.unwrap();
                if point.time > from_time {
                    from_time = point.time;
                }
            } 
        }
        if well.whp_mode == ComputationMode::Client {
            let point_maybe = get_first_point_f32_pool(pool, well, 3).await;
            if point_maybe.is_some() {
                let point = point_maybe.unwrap();
                if point.time > from_time {
                    from_time = point.time;
                }
            } 
        }
        if well.rate_mode == ComputationMode::Client {
            let point_maybe = get_first_point_f32_pool(pool, well, 4).await;
            if point_maybe.is_some() {
                let point = point_maybe.unwrap();
                if point.time > from_time {
                    from_time = point.time;
                }
            } 
        }
        if well.rho_mode == ComputationMode::Client {
            let point_maybe = get_first_point_f32_pool(pool, well, 5).await;
            if point_maybe.is_some() {
                let point = point_maybe.unwrap();
                if point.time > from_time {
                    from_time = point.time;
                }
            } 
        }
        if well.vtot_mode == ComputationMode::Client {
            let point_maybe = get_first_point_f64_pool(pool, well, 6).await;
            if point_maybe.is_some() {
                let point = point_maybe.unwrap();
                if point.time > from_time {
                    from_time = point.time;
                }
            } 
        }
        if well.ii_mode == ComputationMode::Client {
            let point_maybe = get_first_point_f32_pool(pool, well, 7).await;
            if point_maybe.is_some() {
                let point = point_maybe.unwrap();
                if point.time > from_time {
                    from_time = point.time;
                }
            } 
        }
        from_time
    }
}

async fn get_last_vtot_point(tx: &mut Transaction<'_, Postgres>, well: &Well)
        -> Option<PointF64> {
    let last_point_maybe = get_last_point_f64(tx, well, 6).await;
    if last_point_maybe.is_some() {
        last_point_maybe
    } else {
        let rate_point_maybe = get_first_point_f32(tx, well, 4).await;
        if rate_point_maybe.is_some() {
            let rate_point = rate_point_maybe.unwrap();
            let volume_point = PointF64{time: rate_point.time, value: 0.0};
            insert_point_f64(tx, well, 6, volume_point).await;
            Some(volume_point)
        } else {
            None
        }
    }
}

async fn vtot_computer(tx: &mut Transaction<'_, Postgres>, well: &Well, end_to: i32) {
    if well.vtot_mode != ComputationMode::Server {
        return;
    }
    let last_vtot_point_maybe = get_last_vtot_point(tx, well).await;
    if last_vtot_point_maybe.is_none() {
        return;
    }
    let mut last_vtot_point = last_vtot_point_maybe.unwrap();
    if end_to <= last_vtot_point.time {
        return;
    }
    let computation_step: i32 = 30;  // seconds
    let n_volumes: usize = (((end_to - last_vtot_point.time) as f32)/
                              (computation_step as f32)).floor() as usize;
    let mut volumes: Vec<PointF64> = Vec::with_capacity(n_volumes);
    loop {
        let new_time: i32 = last_vtot_point.time + computation_step;
        if new_time > end_to {
            break;
        }
        let rate_points = get_points_f32_lbsrbs_plus_point_before(
            tx, well, 4, last_vtot_point.time, new_time).await;
        let mut dvolume: f64 = 0.0;
        let n_rate_points = rate_points.len();
        if n_rate_points > 0 {
            let mut rate_idx: usize = 1;
            for rate_point in rate_points.iter() {
                // each rate point computes forward until next rate point
                let dt: i32;
                if rate_idx == n_rate_points {
                    if rate_idx == 1 {
                        // single point
                        dt = computation_step
                    } else {
                        dt = new_time - rate_point.time;
                    }
                } else {
                    if rate_idx == 1 {
                        // here at least 2 points by logic
                        dt = rate_points[rate_idx].time - last_vtot_point.time;
                    } else {
                        dt = rate_points[rate_idx].time - rate_points[rate_idx-1].time;
                    }
                }
                dvolume += (rate_point.value as f64) * (dt as f64) / 60.0;
                rate_idx += 1;
            }
        }
        let new_volume = last_vtot_point.value + dvolume;
        let vtot_point = PointF64{time: new_time, value: new_volume};
        volumes.push(vtot_point);
        last_vtot_point = vtot_point;
    }
    insert_points_f64(tx, well, 6, &volumes).await;
}

async fn is_tag_f32_reducing_needed(tx: &mut Transaction<'_, Postgres>,
        well: &Well, tag: &Tag) -> bool {
    // last time
    let last_time: i32;
    {
        let last_point_maybe = get_last_point_f32(tx, well, tag.id).await;
        if last_point_maybe.is_none() {
            return false;
        }
        last_time = last_point_maybe.unwrap().time;
    }
    // last reduced_time
    let last_reduced_time: i32;
    {
        let last_point_maybe = get_last_point_f32(tx, well, -tag.id).await;
        if last_point_maybe.is_some() {
            last_reduced_time = last_point_maybe.unwrap().time;
        } else {
            last_reduced_time = 0;
        }
    }
    if last_reduced_time < last_time {
        return true;
    } else {
        return false;
    }
}

async fn is_tag_f64_reducing_needed(tx: &mut Transaction<'_, Postgres>,
        well: &Well, tag: &Tag) -> bool {
    // last time
    let last_time: i32;
    {
        let last_point_maybe = get_last_point_f64(tx, well, tag.id).await;
        if last_point_maybe.is_none() {
            return false;
        }
        last_time = last_point_maybe.unwrap().time;
    }
    // last reduced_time
    let last_reduced_time: i32;
    {
        let last_point_maybe = get_last_point_f64(tx, well, -tag.id).await;
        if last_point_maybe.is_some() {
            last_reduced_time = last_point_maybe.unwrap().time;
        } else {
            last_reduced_time = 0;
        }
    }
    if last_reduced_time < last_time {
        return true;
    } else {
        return false;
    }
}

async fn reduce_tag_f32(tx: &mut Transaction<'_, Postgres>, well: &Well,
        tag: &Tag, end_to: i32) {
    if !is_tag_f32_reducing_needed(tx, well, tag).await {
        return;
    }
    {
        // ensure a point exists in -tag
        let first_point_maybe = get_first_point_f32(tx, well, -tag.id).await;
        if first_point_maybe.is_none() {
            let point_maybe = get_first_point_f32(tx, well, tag.id).await;
            if point_maybe.is_some() {
                let point = point_maybe.unwrap();
                insert_point_f32(tx, well, -tag.id, point).await;
            } else {
                return;
            }
        }
    }
    let time_from: i32;
    {
        let point_number: i16 = 5;
        let recent_point_maybe = get_recent_point_f32(
            tx, well, -tag.id, point_number).await;
        if recent_point_maybe.is_some() {
            time_from = recent_point_maybe.unwrap().time;
        } else {
           return;
        }
    }
    let time_to: i32;
    {
        let last_point_maybe = get_last_point_f32(tx, well, tag.id).await;
        if last_point_maybe.is_none() {
            return;
        }
        let last_point = last_point_maybe.unwrap();
        if last_point.time > end_to {
            time_to = end_to;
        } else {
            time_to = last_point.time;
        }
    }
    if time_from >= time_to {
        return;
    }
    clear_points_from_nonstrict_f32(tx, well, -tag.id, time_from).await;
    /*
        - 1 month = 60*60*24*30 = 2592000
        - per month, there are 1000 points
        - between points, there are m/1000 = 2592 seconds

        the interval that needs to be reduced is (timeFrom, timeTo]
    */
    let seconds_between_points: f32 = 2592.0; // = m/1000
    let points = get_points_f32_lbnsrbns(tx, well, tag.id, time_from, time_to).await;
    let threshold: usize;
    {
        let dt: f32 = (time_to - time_from) as f32;
        let k = (1.0 + dt/seconds_between_points).floor() as usize;
        if k < 3 {
            threshold = 3;  // minimum threshold for lttb
        } else {
            threshold = k;
        }
    }
    let reduced_points = lttb_f32(&points, threshold);
    insert_points_f32(tx, well, -tag.id, &reduced_points).await;
}

async fn reduce_tag_f64(tx: &mut Transaction<'_, Postgres>, well: &Well,
        tag: &Tag, end_to: i32) {
    if !is_tag_f64_reducing_needed(tx, well, tag).await {
        return;
    }
    {
        // ensure a point exists in -tag
        let first_point_maybe = get_first_point_f64(tx, well, -tag.id).await;
        if first_point_maybe.is_none() {
            let point_maybe = get_first_point_f64(tx, well, tag.id).await;
            if point_maybe.is_some() {
                let point = point_maybe.unwrap();
                insert_point_f64(tx, well, -tag.id, point).await;
            } else {
                return;
            }
        }
    }
    let time_from: i32;
    {
        let point_number: i16 = 5;
        let recent_point_maybe = get_recent_point_f64(
            tx, well, -tag.id, point_number).await;
        if recent_point_maybe.is_some() {
            time_from = recent_point_maybe.unwrap().time;
        } else {
           return;
        }
    }
    let time_to: i32;
    {
        let last_point_maybe = get_last_point_f64(tx, well, tag.id).await;
        if last_point_maybe.is_none() {
            return;
        }
        let last_point = last_point_maybe.unwrap();
        if last_point.time > end_to {
            time_to = end_to;
        } else {
            time_to = last_point.time;
        }
    }
    if time_from >= time_to {
        return;
    }
    clear_points_from_nonstrict_f64(tx, well, -tag.id, time_from).await;
    /*
        - 1 month = 60*60*24*30 = 2592000
        - per month, there are 1000 points
        - between points, there are m/1000 = 2592 seconds

        the interval that needs to be reduced is (timeFrom, timeTo]
    */
    let seconds_between_points: f32 = 2592.0; // = m/1000
    let points = get_points_f64_lbnsrbns(tx, well, tag.id, time_from, time_to).await;
    let threshold: usize;
    {
        let dt: f32 = (time_to - time_from) as f32;
        let k = (1.0 + dt/seconds_between_points).floor() as usize;
        if k < 3 {
            threshold = 3;  // minimum threshold for lttb
        } else {
            threshold = k;
        }
    }
    let reduced_points = lttb_f64(&points, threshold);
    insert_points_f64(tx, well, -tag.id, &reduced_points).await;
}

async fn reduce_data(tx: &mut Transaction<'_, Postgres>, well: &Well, end_to: i32) {
    let tags: [Tag; 7] = [
        Tag{id: 1, value_size: PointValueSize::F32},
        Tag{id: 2, value_size: PointValueSize::F32},
        Tag{id: 3, value_size: PointValueSize::F32},
        Tag{id: 4, value_size: PointValueSize::F32},
        Tag{id: 5, value_size: PointValueSize::F32},
        Tag{id: 6, value_size: PointValueSize::F64},
        Tag{id: 7, value_size: PointValueSize::F32},
    ];
    for tag in tags.iter() {
        if tag.value_size == PointValueSize::F32 {
            reduce_tag_f32(tx, well, tag, end_to).await;
        } else {
            reduce_tag_f64(tx, well, tag, end_to).await;
        }
    }
    let custom_tags = get_custom_tags(tx, well).await;
    for custom_tag in custom_tags.iter() {
        if custom_tag.value_size == PointValueSize::F32 {
            reduce_tag_f32(tx, well, custom_tag, end_to).await;
        } else {
            reduce_tag_f64(tx, well, custom_tag, end_to).await;
        }
    }
}

async fn compute_well(pool: &PgPool, well: &Well) {
    let computable_time_to: i32 = get_computable_time_to(pool, well).await;
    if computable_time_to > well.computed_to {
        let mut from_time: i32 = get_optimised_from_time(pool, well).await;
        if from_time > 0 {
            let computation_step: i32 = 3600;  // 1 hour
            let mut is_end_reached = false;
            loop {
                let mut detached_connection: PgConnection = pool.acquire().await
                    .unwrap().detach();
                let mut tx = detached_connection.begin().await.unwrap();

                let mut step_end_to: i32 = from_time + computation_step;
                if step_end_to >= computable_time_to {
                    step_end_to = computable_time_to;
                    is_end_reached = true;
                }
                // skipped: bhp_computer
                // skipped: bht_computer
                // skipped: whp_computer
                // skipped: rate_computer
                // implement later: density_computer (depends on friction_losses argument)
                vtot_computer(&mut tx, well, step_end_to).await;
                // implement later: ii_computer (depends on previous PRES value)

                update_computed_to(&mut tx, well, step_end_to).await;
                reduce_data(&mut tx, well, step_end_to).await;
                if is_end_reached {
                    update_computer_needed(&mut tx, well, false).await;
                }
                let commit_res = tx.commit().await;
                if commit_res.is_err() {
                    panic!("compute_well: commit failed!");
                }

                if is_end_reached {
                    break;
                }
                from_time += computation_step;
            }
        }
    } else {
        update_computer_needed_pool(pool, well, false).await;
    }
}

#[tokio::main]
async fn main() {
    let db_url_res = env::var("GEOTOOL_COMPUTER_PG_URL");
    if db_url_res.is_err() {
        panic!("GEOTOOL_COMPUTER_PG_URL ENVVAR is missing!");
    }
    let db_url = db_url_res.unwrap();
    let pool_res = PgPoolOptions::new()
        .max_connections(3)
        .connect(&db_url).await;
    if pool_res.is_err() {
        panic!("PG POOL failed to setup!");
    }
    let pool: PgPool = pool_res.unwrap();

    let wait_time = time::Duration::from_secs(30);
    println!("computer started..");
    loop {
        let wells = get_wells(&pool).await;
        for well in wells.iter() {
            if well.computer_needed {
                compute_well(&pool, well).await;
            }
        }
        thread::sleep(wait_time);
    }
}

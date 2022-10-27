use crate::point::{PointF32, PointF64};

fn get_avg_range_start(every: f64, sample: usize) -> usize {
    (every * ((sample - 1) as f64) + 1.0).floor() as usize
}

fn get_avg_range_end(every: f64, sample: usize, n_points: usize) -> usize {
    let preliminary: usize = (every * (sample as f64) + 1.0).floor() as usize;
    if preliminary < n_points {
        preliminary
    } else {
        n_points
    }
}

pub fn lttb_f32(points: &Vec<PointF32>, threshold: usize) -> Vec<PointF32> {
    let n_points = points.len();
    if (threshold < 3) || (threshold >= n_points) {
        return points.to_vec();
    }
    let every = ((n_points - 2) as f64)/((threshold - 2) as f64);
    let mut filtered: Vec<PointF32> = Vec::with_capacity(threshold);
    let mut index_a: usize = 0;
    for sample in 1..=threshold {
        if sample == 1 {
            filtered.push(points[0]);
        } else if sample == threshold {
            filtered.push(points[n_points - 1]);
        } else {
            let avg_range_start = get_avg_range_start(every, sample);
            let avg_range_end = get_avg_range_end(every, sample, n_points);
            let mut avg_time: f64 = 0.0;
            let mut avg_value: f64 = 0.0;
            for elem in avg_range_start..avg_range_end {
                avg_time += points[elem].time as f64;
                avg_value += points[elem].value as f64;
            }
            {
                let subrange_len = (avg_range_end - avg_range_start) as f64;
                avg_time /= subrange_len;
                avg_value /= subrange_len;
            }
            let range_offs = (every * ((sample - 2) as f64) + 1.0).floor() as usize;
            let range_to = (every * ((sample - 1) as f64) + 1.0).floor() as usize;
            let point_a = points[index_a];
            let mut max_area: f64 = -1.0;
            let mut optimal_point = points[range_offs];
            for elem in range_offs..range_to {
                let point = points[elem];
                let part1: f64 = (point_a.time as f64 - avg_time)
                    * ((point.value - point_a.value) as f64);
                let part2: f64 = ((point_a.time - point.time) as f64)
                    * (avg_value - (point_a.value as f64));
                let area = (part1 - part2).abs() * 0.5;
                if area > max_area {
                    max_area = area;
                    index_a = elem;
                    optimal_point = point;
                }
            }
            filtered.push(optimal_point);
        }
    }
    filtered
}

pub fn lttb_f64(points: &Vec<PointF64>, threshold: usize) -> Vec<PointF64> {
    let n_points = points.len();
    if (threshold < 3) || (threshold >= n_points) {
        return points.to_vec();
    }
    let every = ((n_points - 2) as f64)/((threshold - 2) as f64);
    let mut filtered: Vec<PointF64> = Vec::with_capacity(threshold);
    let mut index_a: usize = 0;
    for sample in 1..=threshold {
        if sample == 1 {
            filtered.push(points[0]);
        } else if sample == threshold {
            filtered.push(points[n_points - 1]);
        } else {
            let avg_range_start = get_avg_range_start(every, sample);
            let avg_range_end = get_avg_range_end(every, sample, n_points);
            let mut avg_time: f64 = 0.0;
            let mut avg_value: f64 = 0.0;
            for elem in avg_range_start..avg_range_end {
                avg_time += points[elem].time as f64;
                avg_value += points[elem].value;
            }
            {
                let subrange_len = (avg_range_end - avg_range_start) as f64;
                avg_time /= subrange_len;
                avg_value /= subrange_len;
            }
            let range_offs = (every * ((sample - 2) as f64) + 1.0).floor() as usize;
            let range_to = (every * ((sample - 1) as f64) + 1.0).floor() as usize;
            let point_a = points[index_a];
            let mut max_area: f64 = -1.0;
            let mut optimal_point = points[range_offs];
            for elem in range_offs..range_to {
                let point = points[elem];
                let part1: f64 = (point_a.time as f64 - avg_time)
                    * (point.value - point_a.value);
                let part2: f64 = ((point_a.time - point.time) as f64)
                    * (avg_value - point_a.value);
                let area = (part1 - part2).abs() * 0.5;
                if area > max_area {
                    max_area = area;
                    index_a = elem;
                    optimal_point = point;
                }
            }
            filtered.push(optimal_point);
        }
    }
    filtered
}


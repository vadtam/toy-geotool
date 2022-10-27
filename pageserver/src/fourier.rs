use rustfft::{FftPlanner, num_complex::Complex as Complex};
use crate::point::*;

pub fn water_hammer_fft(points: &Vec<PointF32>) -> Vec<PointF32F32> {
    let n_points = points.len();
    if n_points < 2 {
        let empty_vector: Vec<PointF32F32> = Vec::new();
        return empty_vector;
    }
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(n_points);
    let mut buffer: Vec<Complex<f32>> = Vec::with_capacity(n_points);
    for point in points.iter() {
        buffer.push(Complex{re: point.value, im: 0.0});
    }
    fft.process(&mut buffer);
    let sample_rate: f32;
    {
        let dt = (points[n_points - 1].time - points[0].time) as f32;
        sample_rate = dt / ((n_points - 1) as f32);
    }
    let new_len = buffer.len().wrapping_div(2);
    let mut spectrum: Vec<PointF32F32> = Vec::with_capacity(new_len);
    for idx in 1..=new_len {
        let x: f32 = 1.0 * (idx as f32) / (2.0 * sample_rate * (new_len as f32));
        let y: f32 = 2.0 * buffer[idx].norm() / (n_points as f32);
        spectrum.push(PointF32F32{x: x, y: y});
    }
    spectrum
}



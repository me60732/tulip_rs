//use crate::common::{Inputs, Options, Outputs};
/*pub fn abs(data: &[f64], start_index: usize) -> Vec<f64> {
    data[start_index..].iter().map(|&x| x.abs()).collect()
}

pub fn acos(data: &[f64], start_index: usize) -> Vec<f64> {
    data[start_index..].iter().map(|&x| x.acos()).collect()
}*/

pub fn abs(value: f64) -> f64 {
    value.abs()
}

pub fn acos(value: f64) -> f64 {
    value.acos()
}

pub fn crossover(inputs: &[&[f64]; 2]) -> Vec<f64> {
    let mut result = Vec::new();
    if inputs[0].len() < 2 {
        return result;
    }

    let in1 = &inputs[0];
    let in2 = &inputs[1];

    for i in 1..in1.len() {
        if in1[i - 1] < in2[i - 1] && in1[i] > in2[i] {
            result.push(1.0);
        } else {
            result.push(0.0);
        }
    }

    result
}

pub fn crossany(inputs: &[&[f64]; 2]) -> Vec<f64> {
    let mut result = Vec::new();
    if inputs.len() < 2 || inputs[0].len() < 2 {
        return result;
    }

    let in1 = &inputs[0];
    let in2 = &inputs[1];

    for i in 1..in1.len() {
        if (in1[i - 1] < in2[i - 1] && in1[i] > in2[i]) || (in1[i - 1] > in2[i - 1] && in1[i] < in2[i]) {
            result.push(1.0);
        } else {
            result.push(0.0);
        }
    }

    result
}
pub fn sum(real: &[f64], period: usize) -> f64 {
    real[0..period].iter().sum::<f64>()
}


use tulip_rs::indicators::cvi::{indicator, TIndicatorState};

fn main() {
    // Example data: high and low prices
    let high = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ]; // High prices
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ]; // Low prices
    let options = [5.0];

    let inputs = [high.as_slice(), low.as_slice()];

    // Calculate the CVI indicator
    let (outputs, _) = match indicator(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full CVI values: {:?}", outputs[0]);

    let inputs2 = [&high[0..high.len() - 1], &low[0..low.len() - 1]];

    // Calculate the CVI indicator
    let (outputs2, mut state) = match indicator(&inputs2, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("CVI values: {:?}", outputs2[0]);

    // New high and low prices for the next period
    let new_inputs = [&high[high.len() - 1..], &low[low.len() - 1..]];

    // Calculate the CVI indicator from the previous state
    let new_outputs = match state.batch_indicator(&new_inputs, Some(&[true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nNew CVI values: {:?}", new_outputs[0]);
}

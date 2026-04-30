use tulip_rs::indicators::medprice::{indicator, TIndicatorState};

fn main() {
    // Example input data: high and low prices
    let high = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ]; // High prices
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ]; // Low prices

    let inputs = [&high[..high.len() - 5], &low[..low.len() - 5]];

    // Calculate the MEDPRICE using the indicator function
    let (outputs, mut state) = match indicator(&inputs, &[], None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("MEDPRICE Line: {:?}", outputs[0]);

    let new_data = [&high[high.len() - 5..], &low[low.len() - 5..]];

    // Calculate the new MEDPRICE lines using the previous MEDPRICE values as the starting point
    let new_outputs = match state.batch_indicator(&new_data, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nNew MEDPRICE Line: {:?}", new_outputs[0]);
}

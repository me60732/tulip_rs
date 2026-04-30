use tulip_rs::indicators::dm::{indicator, TIndicatorState};

fn main() {
    let high = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ]; // High prices
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ]; // Low prices

    let inputs = [&high[0..high.len() - 5], &low[0..low.len() - 5]];
    let options = [5.0]; // Period

    // Calculate the Directional Movement (DM) lines using the indicator function
    let (outputs, mut state) = match indicator(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Plus DM: {:?}", outputs[0]);
    println!("Minus DM: {:?}", outputs[1]);

    let new_data = [&high[high.len() - 5..], &low[low.len() - 5..]];

    // Calculate the new DM lines using the previous DM values as the starting point
    let new_outputs = match state.batch_indicator(&new_data, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nNew Plus DM: {:?}", new_outputs[0]);
    println!("New Minus DM: {:?}", new_outputs[1]);
}

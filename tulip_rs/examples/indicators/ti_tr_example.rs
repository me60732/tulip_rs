use tulip_rs::indicators::tr::{indicator, TIndicatorState};

fn main() {
    let high = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ]; // High prices
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ]; // Low prices
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices

    let inputs = [
        &high[0..high.len() - 5],
        &low[0..low.len() - 5],
        &close[0..close.len() - 5],
    ];

    // Calculate the True Range (TR) line using the indicator function
    let (outputs, mut state) = match indicator(&inputs, &[], None) {
        Ok(result) => result,
        Err(e) => {
            println!("Error: {:?}", e);
            return;
        }
    };
    println!("TR Line: {:?}", outputs[0]);

    let new_data = [
        &high[high.len() - 5..],
        &low[low.len() - 5..],
        &close[close.len() - 5..],
    ];

    // Calculate the new TR line using the previous state
    let new_outputs = match state.batch_indicator(&new_data, None) {
        Ok(result) => result,
        Err(e) => {
            println!("Error: {:?}", e);
            return;
        }
    };
    println!("New TR Line: {:?}", new_outputs[0]);
}

use tulip_rs::indicators::rsi::{indicator, TIndicatorState};

fn main() {
    // Example input data: close prices
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices

    let inputs = [&close[..close.len() - 5]];

    // Options for the RSI calculation: period
    let options = [5.0];

    // Calculate the RSI using the indicator function
    let (outputs, mut state) = match indicator(&inputs, &options, Some(&[true, true])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("RSI Line: {:?}", outputs[0]);

    let new_inputs = [&close[close.len() - 5..]];

    let new_outputs = match state.batch_indicator(&new_inputs, Some(&[false, false, false])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("New RSI Line: {:?}", new_outputs[0]);
}

use tulip_rs::indicators::bbands::{indicator, TIndicatorState};

fn main() {
    // Example input data: real prices
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices
    let inputs = [&close[0..close.len() - 5]];

    // Options for the BBANDS calculation: period and standard deviation multiplier
    let options = [5.0, 2.0];

    // Calculate the BBANDS using the indicator function
    let (outputs, mut state) = match indicator(&inputs, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Lower Band: {:?}", outputs[0]);
    println!("Middle Band: {:?}", outputs[1]);
    println!("Upper Band: {:?}", outputs[2]);

    let new_inputs = [&close[close.len() - 5..]];

    let new_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\nNew Lower Band: {:?}", new_outputs[0]);
    println!("New Middle Band: {:?}", new_outputs[1]);
    println!("New Upper Band: {:?}", new_outputs[2]);
}

use tulip_rs::indicators::apo::{indicator, TIndicatorState};

fn main() {
    // Example input data: close prices
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; //close prices
       // Options for the APO calculation: short_period and long_period
    let options = [2.0, 5.0];
    let inputs = [close.as_slice()];

    let (outputs, _) = match indicator(&inputs, &options, Some(&[true, true])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Full APO Line: {:?}", outputs[0]);
    println!("Short EMA Line: {:?}", outputs[1]);
    println!("Long EMA Line: {:?}", outputs[2]);

    let inputs2 = [&close[0..close.len() - 5]];

    // Calculate the APO using the indicator function
    let (outputs2, mut state2) = match indicator(&inputs2, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("APO Line: {:?}", outputs2[0]);

    let new_data = [&close[close.len() - 5..]];

    let new_outputs = match state2.batch_indicator(&new_data, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("New APO Line: {:?}", new_outputs[0]);
}

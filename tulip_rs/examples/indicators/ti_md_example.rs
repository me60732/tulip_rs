use tulip_rs::indicators::md::{indicator, TIndicatorState};

fn main() {
    // Example input data: real prices
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices
       // Options for the MD calculation: period
    let options = [5.0];

    let inputs = [close.as_slice()];
    let (outputs, _) = match indicator(&inputs, &options, Some(&[true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full MD Line: {:?}", outputs[0]);
    println!("Full SMA Line: {:?}", outputs[1]);

    let inputs2 = [&close[0..close.len() - 5]];
    // Calculate the MD using the indicator function
    let (outputs2, mut state) = match indicator(&inputs2, &options, Some(&[true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nMD Line: {:?}", outputs2[0]);
    println!("SMA Line: {:?}", outputs2[1]);

    let new_input = [&close[close.len() - 5..]];

    // Calculate the MD using the previous state
    let new_outputs = match state.batch_indicator(&new_input, Some(&[true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nNew MD Line: {:?}", new_outputs[0]);
    println!("New SMA Line: {:?}", new_outputs[1]);
}

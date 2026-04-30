use tulip_rs::indicators::max::{indicator, TIndicatorState};

fn main() {
    // Example input data (close prices)
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    let inputs = [close.as_slice()];

    // Example options
    let period = 5.0;
    let options = [period];

    // Calculate the max indicator values
    let (outputs, _) = match indicator(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    // Print the results
    println!("Full max Line: {:?}", outputs[0]);

    let inputs2 = [&close[..close.len() - 5]];
    let (outputs2, mut state) = match indicator(&inputs2, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    // Print the results
    println!("\nmax Line: {:?}", outputs2[0]);

    let new_inputs = [&close[close.len() - 5..]];
    // Calculate the max indicator values from the previous state
    let new_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    // Print the results
    println!("New max Line: {:?}", new_outputs[0]);
}

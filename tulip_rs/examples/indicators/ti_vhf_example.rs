use tulip_rs::indicators::vhf::{indicator, TIndicatorState};

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

    // Calculate the vhf indicator values
    let (result, _) = match indicator(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    // Print the results
    println!("Full VHF Line: {:?}", result[0]);

    let inputs2 = [&close[..close.len() - 5]];
    let (result2, mut state2) = match indicator(&inputs2, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial VHF Line: {:?}", result2[0]);

    let new_inputs = [&close[close.len() - 5..]];
    // Calculate the vhf indicator values from the previous state
    let new_result = match state2.batch_indicator(&new_inputs, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    // Print the results
    println!("\nFinal VHF Line: {:?}", new_result[0]);
}

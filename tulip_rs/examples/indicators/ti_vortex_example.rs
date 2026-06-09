use tulip_rs::indicators::vortex::{indicator, TIndicatorState};

fn main() {
    // Test Input Data
    let high = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87, 88.50, 89.20, 89.75, 90.10, 89.80f64,
    ];
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01, 87.60, 88.15, 88.90, 89.40, 88.95f64,
    ];
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29, 88.10, 88.80, 89.50, 89.95, 89.25f64,
    ];
    let period = 5.0;
    let options = [period];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

    // Calculate the Vortex Indicator using the full dataset (no optional outputs)
    let (outputs, _) = match indicator(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full VI+ Line: {:?}", outputs[0]);
    println!("Full VI- Line: {:?}", outputs[1]);

    // Calculate again with optional TR output enabled
    let (outputs_with_tr, _) = match indicator(&inputs, &options, Some(&[true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nFull VI+ Line (with TR): {:?}", outputs_with_tr[0]);
    println!("Full VI- Line (with TR): {:?}", outputs_with_tr[1]);
    println!("Full TR Line:            {:?}", outputs_with_tr[2]);

    let n = high.len() - 5;
    let inputs2 = [&high[..n], &low[..n], &close[..n]];

    // Calculate the Vortex Indicator using a partial dataset
    let (outputs2, mut state) = match indicator(&inputs2, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial VI+ Line: {:?}", outputs2[0]);
    println!("Partial VI- Line: {:?}", outputs2[1]);

    // Use the last 5 inputs to continue from the saved state
    let new_inputs = [&high[n..], &low[n..], &close[n..]];

    // Calculate the Vortex Indicator using the recent data and previous state
    let new_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nContinued VI+ Line: {:?}", new_outputs[0]);
    println!("Continued VI- Line: {:?}", new_outputs[1]);
}

use tulip_rs::indicators::linreg::{indicator, TIndicatorState};

fn main() {
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];
    let options = [5.0]; // Period

    let inputs = [close.as_slice()];
    let (outputs, _) = match indicator(&inputs, &options, Some(&[true, true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full Linreg Line: {:?}", outputs[0]);
    println!("Full Slope Line: {:?}", outputs[1]);
    println!("Full Intercept Line: {:?}", outputs[2]);

    let inputs2 = [&close[0..close.len() - 5]];

    // Example with recent_only parameter set to false
    let (outputs2, mut state) = match indicator(&inputs2, &options, Some(&[true, true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial Linreg Line: {:?}", outputs2[0]);
    println!("Partial Slope Line: {:?}", outputs2[1]);
    println!("Partial Intercept Line: {:?}", outputs2[2]);

    let new_inputs = [&close[close.len() - 5..]];

    let new_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nFinal LINREG Line: {:?}", new_outputs[0]);
}

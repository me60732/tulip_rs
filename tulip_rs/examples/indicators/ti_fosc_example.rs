use tulip_rs::indicators::fosc::{indicator, TIndicatorState};

fn main() {
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];
    let options = [5.0]; // Period

    let inputs = [close.as_slice()];

    // Example with recent_only parameter set to false
    let (outputs, _) = match indicator(&inputs, &options, Some(&[true, true, true, true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Full FOSC Line: {:?}", outputs[0]);
    println!("Full TSF Line: {:?}", outputs[1]);
    println!("Full Linreg Line: {:?}", outputs[2]);
    println!("Full Slope Line: {:?}", outputs[3]);
    println!("Full Intercept Line: {:?}", outputs[4]);

    let new_inputs = [&close[..close.len() - 5]];
    let final_inputs = [&close[close.len() - 5..]];

    let (outputs, mut state) = match indicator(&new_inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\nPartial FOSC Line: {:?}", outputs[0]);

    let outputs = match state.batch_indicator(&final_inputs, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\nFinal FOSC Line: {:?}", outputs[0]);
}

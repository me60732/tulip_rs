use tulip_rs::indicators::trix::{indicator, TIndicatorState};

fn main() {
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices
    let inputs = [close.as_slice()];

    let options = [5.0]; // Period
    let (outputs, _) = match indicator(&inputs, &options, Some(&[true, true, true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full Trix Line: {:?}", outputs[0]);
    println!("Tema Line: {:?}", outputs[1]);
    println!("Dema Line: {:?}", outputs[2]);
    println!("Ema Line: {:?}", outputs[3]);

    let inputs2 = [&close[..close.len() - 1]];
    // Example with recent_only parameter set to false
    let (outputs2, mut state) = match indicator(&inputs2, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nTrix Line: {:?}", outputs2[0]);

    let new_inputs = [&close[close.len() - 1..]];

    let new_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nNew Trix Line: {:?}", new_outputs[0]);
}

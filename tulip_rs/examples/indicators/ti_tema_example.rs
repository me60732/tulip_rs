use tulip_rs::indicators::tema::{indicator, TIndicatorState};

fn main() {
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices
    let options = [5.0]; // Period

    let inputs = [close.as_slice()];

    let (outputs, _) = match indicator(&inputs, &options, Some(&[true, true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full TEMA Line: {:?}", outputs[0]);
    println!("Full EMA Line: {:?}", outputs[1]);
    println!("Full DEMA Line: {:?}", outputs[2]);

    let inputs2 = [&close[..close.len() - 1]];

    // Example with recent_only parameter set to false
    let (outputs2, mut state) = match indicator(&inputs2, &options, Some(&[true, true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial TEMA Line: {:?}", outputs2[0]);
    println!("Partial EMA Line: {:?}", outputs2[1]);
    println!("Partial DEMA Line: {:?}", outputs2[2]);

    let new_inputs = [&close[close.len() - 1..]];

    let new_outputs = match state.batch_indicator(&new_inputs, Some(&[true, true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nNew TEMA Line: {:?}", new_outputs[0]);
    println!("New EMA Line: {:?}", new_outputs[1]);
    println!("New DEMA Line: {:?}", new_outputs[2]);
}

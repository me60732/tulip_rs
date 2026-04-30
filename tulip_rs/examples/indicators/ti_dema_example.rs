use tulip_rs::indicators::dema::{indicator, TIndicatorState};

fn main() {
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices

    let inputs = [&close[0..close.len() - 1]];
    let options = [5.0]; // Period

    // Example with recent_only parameter set to false
    let (outputs, mut state) = match indicator(&inputs, &options, Some(&[true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("DEMA Line: {:?}", outputs[0]);
    println!("EMA Line: {:?}", outputs[1]);

    let new_inputs = [&close[close.len() - 1..]];

    let new_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nNew DEMA Line: {:?}", new_outputs[0]);
}

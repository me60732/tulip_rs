use tulip_rs::indicators::wma::{indicator, TIndicatorState};

fn main() {
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices

    let inputs = [&close[..close.len() - 5]];
    let options = [5.0];

    let (outputs, mut state) = match indicator(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    // WMA only has one output
    println!("WMA Line: {:?}", outputs[0]);

    let new_inputs = [&close[close.len() - 5..]];

    let new_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("New WMA Line: {:?}", new_outputs[0]);
}

use tulip_rs::indicators::qstick::{indicator, TIndicatorState};

fn main() {
    let open = [
        81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18,
        88.00, 87.60,
    ]; // Open prices
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices

    let inputs = [&open[0..open.len() - 5], &close[0..close.len() - 5]];
    let options = [5.0];
    let (outputs, mut state) = match indicator(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("QStick Line: {:?}", outputs[0]);

    let new_inputs = [&open[open.len() - 5..], &close[close.len() - 5..]];

    let new_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("New QStick Line: {:?}", new_outputs[0]);
}

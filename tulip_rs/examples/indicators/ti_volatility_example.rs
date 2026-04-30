use tulip_rs::indicators::volatility::{indicator, TIndicatorState};
fn main() {
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices
    let options = [5.0];

    let inputs = [close.as_slice()];

    let (outputs, _) = match indicator(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Full Volatility Line: {:?}", outputs[0]);

    let inputs2 = [&close[..close.len() - 5]];

    let (outputs2, mut state) = match indicator(&inputs2, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\nPartial Volatility Line: {:?}", outputs2[0]);

    let inputs3 = [&close[close.len() - 5..]];
    let new_outputs = match state.batch_indicator(&inputs3, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\nFinal Volatility Line: {:?}", new_outputs[0]);
}

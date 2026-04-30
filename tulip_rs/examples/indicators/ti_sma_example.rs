use tulip_rs::indicators::sma::{indicator, TIndicatorState};
fn main() {
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices
    let options = [5.0];

    let close_vec = close.to_vec();
    let inputs = [close_vec.as_slice()];

    /////////////////////////////////////////////////// Calculating the Full sma Line
    let (outputs, _) = match indicator(&inputs, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full SMA Line: {:?}", outputs[0]);

    /////////////////////////////////////////////////// Calculating the partial sma Line
    let close_vec2 = &close[..close.len() - 5];
    let inputs2 = [close_vec2];

    let (outputs2, mut state2) = match indicator(&inputs2, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Partial SMA Line: {:?}", outputs2[0]);

    /////////////////////////////////////////////////// Calculating the sma Line from state
    let new_close_vec = &close[close.len() - 5..];
    let new_data = [new_close_vec];
    let final_outputs = match state2.batch_indicator(&new_data, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Final SMA Line: {:?}", final_outputs[0]);
}

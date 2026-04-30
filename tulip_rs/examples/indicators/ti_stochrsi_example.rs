use tulip_rs::indicators::stochrsi::{indicator, TIndicatorState};

fn main() {
    // Close column from Context-chat-gpt.txt file
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];
    let options = [5.0];

    let inputs = [close.as_slice()];

    // Calculate Stoch RSI
    let (outputs, _) = match indicator(&inputs, &options, Some(&[true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full Stoch RSI: {:?}", outputs[0]);
    println!("Full RSI: {:?}", outputs[1]);

    let inputs2 = [&close[..close.len() - 4]];

    let (outputs2, mut state) = match indicator(&inputs2, &options, Some(&[true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial Stoch RSI: {:?}", outputs2[0]);
    println!("Partial RSI: {:?}", outputs2[1]);

    let inputs3 = [&close[close.len() - 4..]];
    let new_outputs = match state.batch_indicator(&inputs3, Some(&[true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nContinuation Stoch RSI: {:?}", new_outputs[0]);
    println!("Continuation RSI: {:?}", new_outputs[1]);
}

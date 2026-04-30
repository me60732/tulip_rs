use tulip_rs::indicators::ppo::{indicator, TIndicatorState};

fn main() {
    // Example input data: close prices
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices
    // Options for the PPO calculation: short_period and long_period
    let options = [2.0, 5.0];
    
    let inputs = [close.as_slice()];
    let (outputs, _) = match indicator(&inputs, &options, Some(&[true, true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("Full PPO Line: {:?}", outputs[0]);
    println!("Full short ema Line: {:?}", outputs[1]);
    println!("Full long ema Line: {:?}", outputs[2]);
    
    let inputs = [&close[..close.len() - 5]];

    // Calculate the PPO using the indicator function
    let (outputs, mut state) = match indicator(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("PPO Line: {:?}", outputs[0]);

    // Prepare data for indicator_from_state
    let new_input = [&close[close.len() - 5..]];

    // Calculate the PPO using the previous state
    let new_outputs = match state.batch_indicator(&new_input, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nNew PPO Line: {:?}", new_outputs[0]);
}

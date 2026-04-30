use tulip_rs::indicators::pvi::{indicator, TIndicatorState};

fn main() {
    // Example input data: close prices and volume
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices
    let volume = [
        5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
        4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
    ]; // Volume
    let inputs = [&close[..close.len() - 5], &volume[..volume.len() - 5]];

    // Options for the pvi calculation: no options needed
    let options = [];

    // Calculate the pvi using the indicator function
    let (outputs, mut state) = match indicator(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("pvi Line: {:?}", outputs[0]);

    let new_input = [&close[close.len() - 5..], &volume[volume.len() - 5..]];

    // Calculate the pvi using the previous state
    let new_outputs = match state.batch_indicator(&new_input, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nNew pvi Line: {:?}", new_outputs[0]);
}

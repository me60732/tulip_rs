use tulip_rs::indicators::adosc::{indicator, TIndicatorState};
fn main() {
    let high = [
        82.15, 81.89, 83.03, 83.3, 83.85, 83.9, 83.33, 84.3, 84.84, 85.0, 85.9, 86.58, 86.98, 88.0,
        87.87,
    ]; // High prices
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.3, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ]; // Low prices
    let close = [
        81.59, 81.06, 82.87, 83.0, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices
    let volume = [
        5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
        4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
    ]; // Volume
    let options = [2.0, 5.0]; // Short period and long period
    let inputs = [
        high.as_slice(),
        low.as_slice(),
        close.as_slice(),
        volume.as_slice(),
    ];

    // Call the indicator function
    let (outputs, _) = match indicator(&inputs, &options, Some(&[true, true, true])) {
        Ok(results) => results,
        Err(e) => {
            println!("Error: {:?}", e.message());
            return;
        }
    };

    // Process the results
    println!("Full ADOSC Line: {:?}", outputs[0]);
    println!("\nFull Short_EMA Line: {:?}", outputs[1]);
    println!("\nFull Long_EMA Line: {:?}", outputs[2]);
    println!("\nFull AD Line: {:?}", outputs[3]);

    let inputs = [
        &high[0..high.len() - 5],
        &low[0..low.len() - 5],
        &close[0..close.len() - 5],
        &volume[0..volume.len() - 5],
    ];

    // Call the indicator function
    let (outputs, mut state) = match indicator(&inputs, &options, Some(&[true, true, true])) {
        Ok(results) => results,
        Err(e) => {
            println!("Error: {:?}", e.message());
            return;
        }
    };

    // Process the results
    println!("\n\nPartial ADOSC Line: {:?}", outputs[0]);

    let new_high_vec = &high[high.len() - 5..];
    let new_low_vec = &low[low.len() - 5..];
    let new_close_vec = &close[close.len() - 5..];
    let new_volume_vec = &volume[volume.len() - 5..];
    let new_data = [new_high_vec, new_low_vec, new_close_vec, new_volume_vec];

    let outputs = match state.batch_indicator(&new_data, None) {
        Ok(results) => results,
        Err(e) => {
            println!("Error: {:?}", e.message());
            return;
        }
    };

    println!("\n\nFinal ADOSC Line: {:?}", outputs[0]);
}

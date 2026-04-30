use tulip_rs::candle_indicators::bearishbelthold::{indicator, indicator_from_state, info};

fn main() {
    // Example input data: open, high, low, and close prices
    let open = vec![
        81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18,
        88.00, 87.60,
    ]; // Open prices
    let high = vec![
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ]; // High prices
    let low = vec![
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ]; // Low prices
    let close = vec![
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices

    let options = [7.0, 5.0, 70.0, 0.5, 3.0];
    let info = info();

    // Pattern data that creates a bearish belthold pattern
    let pattern_open = vec![88.50];
    let pattern_high = vec![88.50];
    let pattern_low = vec![87.25];
    let pattern_close = vec![87.50];

    /////////////////////////////////////////////////// Calculating the Full Bearish Belthold Pattern
    let inputs = [
        open.as_slice(),
        high.as_slice(),
        low.as_slice(),
        close.as_slice(),
    ];
    let result = match indicator(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!(
        "Full {} ({}): {:?}",
        info.full_name, info.japanese_name, result.indicators[0]
    );

    /////////////////////////////////////////////////// Calculating the partial Bearish Belthold Pattern
    let partial_len = open.len() - 5;
    let partial_open = &open[..partial_len];
    let partial_high = &high[..partial_len];
    let partial_low = &low[..partial_len];
    let partial_close = &close[..partial_len];

    let partial_inputs = [partial_open, partial_high, partial_low, partial_close];
    let partial_result = match indicator(&partial_inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!(
        "Partial {} ({}): {:?}",
        info.full_name, info.japanese_name, partial_result.indicators[0]
    );

    /////////////////////////////////////////////////// Calculating the Bearish Belthold Pattern from state
    let remaining_open = &open[partial_len..];
    let remaining_high = &high[partial_len..];
    let remaining_low = &low[partial_len..];
    let remaining_close = &close[partial_len..];

    // Extend with pattern data to create bearish belthold
    let mut new_open = remaining_open.to_vec();
    let mut new_high = remaining_high.to_vec();
    let mut new_low = remaining_low.to_vec();
    let mut new_close = remaining_close.to_vec();

    new_open.extend(pattern_open.iter());
    new_high.extend(pattern_high.iter());
    new_low.extend(pattern_low.iter());
    new_close.extend(pattern_close.iter());

    let new_inputs = [
        new_open.as_slice(),
        new_high.as_slice(),
        new_low.as_slice(),
        new_close.as_slice(),
    ];

    let final_result =
        match indicator_from_state(&new_inputs, &options, &partial_result.state, None) {
            Ok(r) => r,
            Err(e) => panic!("Error: {}", e),
        };

    println!(
        "Final {} ({}): {:?}",
        info.full_name, info.japanese_name, final_result.indicators[0]
    );
}

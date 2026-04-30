use tulip_rs::candle_indicators::downsidetasukigap::{indicator, indicator_from_state, info};

fn main() {
    // Example input data: open, high, low, and close prices
    let open = vec![
        81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18,
        88.00, 87.60,
    ];
    let high = vec![
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ];
    let low = vec![
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ];
    let close = vec![
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    let pattern_open = vec![87.60, 86.50, 86.20];
    let pattern_high = vec![87.65, 86.60, 86.80];
    let pattern_low = vec![86.90, 86.10, 86.15];
    let pattern_close = vec![87.00, 86.15, 86.70];

    let info = info();
    let options = [7.0, 5.0, 70.0, 0.5, 3.0];

    // Step 1: Full calculation
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
        "{} ({}): {:?}",
        info.full_name, info.japanese_name, result.indicators[0]
    );

    // Step 2: Partial calculation (exclude last 5 candles)
    let partial_len = open.len() - 5;
    let partial_inputs = [
        &open[..partial_len],
        &high[..partial_len],
        &low[..partial_len],
        &close[..partial_len],
    ];
    let partial_result = match indicator(&partial_inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    // Step 3: Continue from state with remaining data + pattern
    let mut remaining_open = open[partial_len..].to_vec();
    let mut remaining_high = high[partial_len..].to_vec();
    let mut remaining_low = low[partial_len..].to_vec();
    let mut remaining_close = close[partial_len..].to_vec();

    remaining_open.extend(&pattern_open);
    remaining_high.extend(&pattern_high);
    remaining_low.extend(&pattern_low);
    remaining_close.extend(&pattern_close);

    let new_inputs = [
        remaining_open.as_slice(),
        remaining_high.as_slice(),
        remaining_low.as_slice(),
        remaining_close.as_slice(),
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

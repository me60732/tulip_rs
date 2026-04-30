use tulip_rs::indicators::aroon::{indicator, TIndicatorState};
use tulip_rs::indicators::simd_indicators::by_asset::aroon::indicator_by_assets;

const HIGH: [f64; 15] = [
    82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
    88.00, 87.87,
];

const LOW: [f64; 15] = [
    81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
    87.17, 87.01,
];

//const OPTIONS_LIST: [[f64; 1]; 6] = [[5.0], [8.0], [14.0], [25.0], [50.0], [100.0]];

fn expand_inputs() -> (Vec<f64>, Vec<f64>) {
    let mut high_vec = HIGH.to_vec();
    let mut low_vec = LOW.to_vec();
    for _ in 0..300 {
        high_vec.extend_from_slice(&HIGH);
        low_vec.extend_from_slice(&LOW);
    }
    (high_vec, low_vec)
}

fn main() {
    println!("=== Running Regular AROON Example ===\n");
    regular_example();

    println!("\n=== Running SIMD By-Asset Example ===\n");
    simd_by_asset_example();

}

fn regular_example() {
    // Example input data: high and low prices
    let high = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ]; // High prices
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ]; // Low prices

    let inputs = [high.as_slice(), low.as_slice()];

    // Options for the Aroon calculation: period
    let options = [5.0];

    // Calculate the Aroon using the indicator function
    let (outputs, _) = match indicator(&inputs, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Full Aroon Down Line: {:?}", outputs[0]);
    println!("Full Aroon Up Line: {:?}", outputs[1]);

    let inputs2 = [&high[..high.len() - 5], &low[..low.len() - 5]];

    let (outputs2, mut state2) = match indicator(&inputs2, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nAroon Down Line: {:?}", outputs2[0]);
    println!("Aroon Up Line: {:?}", outputs2[1]);

    let new_inputs = [&high[high.len() - 5..], &low[low.len() - 5..]];

    let new_outputs = match state2.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nNew Aroon Down Line: {:?}", new_outputs[0]);
    println!("New Aroon Up Line: {:?}", new_outputs[1]);
}

fn simd_by_asset_example() {
    // Create 4 stocks with different high/low data (representing 4 different assets)
    let (stock1_high, stock1_low) = expand_inputs();
    let (mut stock2_high, mut stock2_low) = expand_inputs();
    let (mut stock3_high, mut stock3_low) = expand_inputs();
    let (mut stock4_high, mut stock4_low) = expand_inputs();

    // Apply different multipliers to each stock to make them unique
    for val in stock2_high.iter_mut() {
        *val *= 1.05; // Stock 2: 5% higher prices
    }
    for val in stock2_low.iter_mut() {
        *val *= 1.05;
    }

    for val in stock3_high.iter_mut() {
        *val *= 0.95; // Stock 3: 5% lower prices
    }
    for val in stock3_low.iter_mut() {
        *val *= 0.95;
    }

    for val in stock4_high.iter_mut() {
        *val *= 1.10; // Stock 4: 10% higher prices
    }
    for val in stock4_low.iter_mut() {
        *val *= 1.10;
    }

    // Create input structure: array of references to arrays of slices
    // Format: [stock][input_field] where input_fields are [high, low]
    let stock1_inputs = [stock1_high.as_slice(), stock1_low.as_slice()];
    let stock2_inputs = [stock2_high.as_slice(), stock2_low.as_slice()];
    let stock3_inputs = [stock3_high.as_slice(), stock3_low.as_slice()];
    let stock4_inputs = [stock4_high.as_slice(), stock4_low.as_slice()];

    let inputs: [&[&[f64]; 2]; 4] = [
        &stock1_inputs,
        &stock2_inputs,
        &stock3_inputs,
        &stock4_inputs,
    ];

    // Set options (period = 5)
    let options = [5.0];

    // Calculate AROON indicator for all 4 stocks in parallel using SIMD
    let (outputs, _states) = match indicator_by_assets::<4>(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!(
        "SIMD By Assets - calculated AROON for {} stocks simultaneously",
        inputs.len()
    );

    // Print first few results for each stock
    for (stock_idx, stock_results) in outputs.iter().enumerate() {
        let aroon_down = &stock_results[0]; // Aroon Down line
        let aroon_up = &stock_results[1]; // Aroon Up line

        println!(
            "Stock {}: Aroon Down first 5: [{:.1}, {:.1}, {:.1}, {:.1}, {:.1}], last: {:.1}",
            stock_idx + 1,
            aroon_down.get(0).unwrap_or(&0.0),
            aroon_down.get(1).unwrap_or(&0.0),
            aroon_down.get(2).unwrap_or(&0.0),
            aroon_down.get(3).unwrap_or(&0.0),
            aroon_down.get(4).unwrap_or(&0.0),
            aroon_down.last().unwrap_or(&0.0)
        );

        println!(
            "Stock {}: Aroon Up first 5: [{:.1}, {:.1}, {:.1}, {:.1}, {:.1}], last: {:.1}",
            stock_idx + 1,
            aroon_up.get(0).unwrap_or(&0.0),
            aroon_up.get(1).unwrap_or(&0.0),
            aroon_up.get(2).unwrap_or(&0.0),
            aroon_up.get(3).unwrap_or(&0.0),
            aroon_up.get(4).unwrap_or(&0.0),
            aroon_up.last().unwrap_or(&0.0)
        );
    }
}

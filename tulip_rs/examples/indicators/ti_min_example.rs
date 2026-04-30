use tulip_rs::indicators::min::{indicator, TIndicatorState, indicator_by_assets, indicator_by_options};

/*const CLOSE: [f64; 15] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29,
];*/
const CLOSE: [f64; 20] = [4.3879, 4.3324, 4.3805, 4.4249, 4.2768, 4.2879, 4.5545, 4.6656, 4.5271, 4.3805, 4.4435, 4.4657, 4.4472, 4.4879, 4.4064, 4.3879, 4.3361, 4.3064, 4.3253, 4.3016];
const OPTIONS_LIST: [[f64; 1]; 8] = [
    [5.0],
    //[7.0],
    [8.0],
    [10.0],
    [14.0],
    [25.0],
    [35.0],
    [50.0],
    [100.0],
];

fn expand_close() -> Vec<f64> {
    let mut close_vec = CLOSE.to_vec();
    for _ in 0..300 {
        close_vec.extend_from_slice(&CLOSE);
    }
    close_vec
}

fn main() {
    println!("=== Running Regular MIN Example ===\n");
    regular_example();

    println!("\n=== Running SIMD By-Asset Debug ===\n");
    simd_by_asset_debug();

    println!("\n=== Running SIMD By-Options Example ===\n");
    simd_by_options_example();

    println!("\n=== Running SIMD By-Asset Example with profiling loop ===\n");

}

fn regular_example() {
    // Example input data (close prices)
    let close = expand_close();

    let inputs = [close.as_slice()];

    // Example options
    let period = 5.0;
    let options = [period];

    // Calculate the min indicator values
    let (outputs, _) = match indicator(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    // Print the results
    println!("Full Min Line: {:?}", outputs[0]);

    let inputs2 = [&close[..close.len() - 5]];
    let (outputs2, mut state) = match indicator(&inputs2, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    // Print the results
    println!("\nMin Line: {:?}", outputs2[0]);

    let new_inputs = [&close[close.len() - 5..]];
    // Calculate the min indicator values from the previous state
    let new_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    // Print the results
    println!("New min Line: {:?}", new_outputs[0]);
}

fn simd_by_asset_example_silent() {
    // Create 4 stocks with different data (representing 4 different assets)
    let stock1 = expand_close();
    let mut stock2 = expand_close();
    let mut stock3 = expand_close();
    let mut stock4 = expand_close();

    // Apply different multipliers to each stock to make them unique
    for val in stock2.iter_mut() {
        *val *= 1.05; // Stock 2: 5% higher prices
    }
    for val in stock3.iter_mut() {
        *val *= 0.95; // Stock 3: 5% lower prices
    }
    for val in stock4.iter_mut() {
        *val *= 1.10; // Stock 4: 10% higher prices
    }

    // Create input structure: array of references to arrays of slices
    // Format: [stock][input_field] where input_field is [close_prices]
    let stock1_inputs = [stock1.as_slice()];
    let stock2_inputs = [stock2.as_slice()];
    let stock3_inputs = [stock3.as_slice()];
    let stock4_inputs = [stock4.as_slice()];

    let inputs: [&[&[f64]; 1]; 4] = [
        &stock1_inputs,
        &stock2_inputs,
        &stock3_inputs,
        &stock4_inputs,
    ];

    // Set options (period = 14)
    let options = [25.0];

    // Calculate min indicator for all 4 stocks in parallel using SIMD
    let (outputs, _states) = match indicator_by_assets::<4>(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    // Use black_box to prevent compiler from optimizing away the computation
    std::hint::black_box(&outputs);
}

fn simd_by_options_example() {
    // Create one stock with expanded close data
    let close = expand_close();

    // Create input structure for single stock
    let inputs = close.as_slice();

    // Use all options from OPTIONS_LIST
    let options: [&[f64; 1]; 8] = [
        &OPTIONS_LIST[0], // [5.0]
        &OPTIONS_LIST[1], // [8.0]
        &OPTIONS_LIST[2], // [10.0]
        &OPTIONS_LIST[3], // [14.0]
        &OPTIONS_LIST[4], // [25.0]
        &OPTIONS_LIST[5], // [35.0]
        &OPTIONS_LIST[6], // [50.0]
        &OPTIONS_LIST[7], // [100.0]
    ];

    // Calculate min indicator for all options in parallel using SIMD
    let (outputs, _states) = match indicator_by_options::<8>(&[&inputs], &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!(
        "SIMD By Options - calculated {} different periods simultaneously",
        options.len()
    );

    // Print first few results for each period
    for (i, &period) in options.iter().enumerate() {
        let results = &outputs[i][0]; // Get the min line for this option
        println!(
            "Period {}: First 5 values: [{:.4}, {:.4}, {:.4}, {:.4}, {:.4}], Last value: {:.4}",
            period[0],
            results.get(0).unwrap_or(&0.0),
            results.get(1).unwrap_or(&0.0),
            results.get(2).unwrap_or(&0.0),
            results.get(3).unwrap_or(&0.0),
            results.get(4).unwrap_or(&0.0),
            results.last().unwrap_or(&0.0)
        );
    }
}

fn simd_by_asset_debug() {
    // Simple debug case with the original small dataset
    let close = expand_close();

    let stock1_inputs = [close.as_slice()];
    let stock2_inputs = [close.as_slice()];
    let stock3_inputs = [close.as_slice()];
    let stock4_inputs = [close.as_slice()];

    let inputs: [&[&[f64]; 1]; 4] = [
        &stock1_inputs,
        &stock2_inputs,
        &stock3_inputs,
        &stock4_inputs,
    ];

    let options = [5.0];

    println!("=== SIMD By Assets Debug ===");
    /*println!(
        "Input close (first 20): {:?}",
        &CLOSE[..20.min(CLOSE.len())]
    );
    println!("Expanded data size: {}", close.len());
    println!("Options: {:?}", options);*/

    // Calculate SIMD MIN
    let (simd_outputs, _) = match indicator_by_assets::<4>(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("SIMD Error: {}", e),
    };

    // Calculate regular MIN for comparison
    let regular_inputs = [close.as_slice()];
    let (regular_outputs, _) = match indicator(&regular_inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Regular Error: {}", e),
    };

    println!("\n=== Results Comparison ===");

    // Check if they match
    let min_match = regular_outputs[0]
        .iter()
        .zip(simd_outputs[0][0].iter())
        .all(|(r, s)| (r - s).abs() < 1e-10);

    if min_match {
        println!("✓ SIMD and regular MIN results match!");
        println!("First 10 values match: {:?}", &regular_outputs[0][..10]);
        println!(
            "Last 10 values match: {:?}",
            &regular_outputs[0][regular_outputs[0].len() - 10..]
        );
    } else {
        println!("❌ SIMD and regular MIN results do NOT match!");

        // Print detailed differences
        println!("\nMIN differences:");
        let mut diff_count = 0;
        for (i, (r, s)) in regular_outputs[0]
            .iter()
            .zip(simd_outputs[0][0].iter())
            .enumerate()
        {
            if (r - s).abs() >= 1e-10 {
                println!(
                    "  Index {}: Regular={:.6}, SIMD={:.6}, Diff={:.6}",
                    i,
                    r,
                    s,
                    r - s
                );
                diff_count += 1;
                if diff_count >= 20 {
                    println!("  ... (showing first 20 differences)");
                    break;
                }
            }
        }
        println!("Total differences: {}", diff_count);

        // Print first few values for comparison
        println!("\nFirst 20 Regular values: {:?}", &regular_outputs[0][..20]);
        println!("First 20 SIMD values: {:?}", &simd_outputs[0][0][..20]);
    }
}

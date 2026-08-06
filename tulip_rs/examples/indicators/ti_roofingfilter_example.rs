use tulip_rs::indicators::roofingfilter::{
    indicator_by_assets, indicator_by_options, Indicator, RoofingFilter, TIndicatorState,
};

// 80 bars of close prices
const CLOSE: [f64; 80] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29, 87.99, 89.17, 88.71, 88.00, 89.11, 89.87, 90.65, 90.31, 89.56, 88.44, 87.23,
    86.55, 85.77, 86.33, 87.50, 88.43, 89.62, 90.54, 91.23, 92.00, 91.50, 90.77, 89.90, 88.55,
    87.66, 86.44, 85.33, 84.55, 85.88, 86.90, 87.45, 88.10, 89.22, 90.35, 91.00, 91.87, 92.44,
    93.10, 92.55, 91.88, 90.66, 89.55, 88.44, 87.33, 86.55, 85.77, 86.44, 87.55, 88.66, 89.77,
    90.33, 91.00, 91.77, 92.44, 93.00, 93.55, 92.88, 91.77, 90.66, 89.55, 88.44, 87.33, 86.55,
    87.44, 88.66,
];

fn main() {
    // Ehlers Roofing Filter: High Pass → Super Smoother cascade.
    // Band-limits price to cycles between ss_period and hp_period bars.
    // Options: [ss_period, hp_period]
    // min_data = max(ss_period, hp_period) + 1
    let options = [10.0, 48.0]; // ss_period = 10, hp_period = 48

    let inputs = [CLOSE.as_slice()];

    // --- Full run with optional highpass output enabled ---
    let (outputs, _) = match RoofingFilter::indicator(&inputs, &options, Some(&[true])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Roofing Filter output ({} values):", outputs[0].len());
    println!("  First 5: {:?}", &outputs[0][..5]);
    println!("  Last  5: {:?}", &outputs[0][outputs[0].len() - 5..]);

    println!(
        "\nHigh Pass intermediate output ({} values):",
        outputs[1].len()
    );
    println!("  First 5: {:?}", &outputs[1][..5]);
    println!("  Last  5: {:?}", &outputs[1][outputs[1].len() - 5..]);

    // --- Streaming / continuation example ---
    let split = CLOSE.len() - 5;
    let inputs_partial = [&CLOSE[..split]];

    let (outputs_partial, mut state) =
        match RoofingFilter::indicator(&inputs_partial, &options, None) {
            Ok(result) => result,
            Err(e) => panic!("Error: {}", e),
        };

    println!(
        "\nPartial roofing (first {} bars, {} values):",
        split,
        outputs_partial[0].len()
    );
    println!(
        "  Last 5: {:?}",
        &outputs_partial[0][outputs_partial[0].len() - 5..]
    );

    let new_inputs = [&CLOSE[split..]];
    let continuation = match state.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Continuation (last 5 bars): {:?}", continuation[0]);
    println!("\n(The continuation values should match the tail of the full run above.)");

    /////////////////////// SIMD by-assets: 4 assets, same options ///////////////////////
    // options = [ss_period=10, hp_period=48] → min_data=49 → 32 output values per asset
    let asset0: [&[f64]; 1] = [CLOSE.as_slice()];
    let close1: Vec<f64> = CLOSE.iter().map(|v| v * 1.2).collect();
    let asset1: [&[f64]; 1] = [close1.as_slice()];
    let close2: Vec<f64> = CLOSE
        .iter()
        .enumerate()
        .map(|(i, v)| 90.0 + i as f64 * 0.5 + v * 0.1)
        .collect();
    let asset2: [&[f64]; 1] = [close2.as_slice()];
    let close3: Vec<f64> = CLOSE
        .iter()
        .enumerate()
        .map(|(i, v)| 100.0 - i as f64 * 0.3 + v * 0.05)
        .collect();
    let asset3: [&[f64]; 1] = [close3.as_slice()];
    let inputs_4: [&[&[f64]; 1]; 4] = [&asset0, &asset1, &asset2, &asset3];

    let (simd_asset_outputs, _) = match indicator_by_assets::<4>(&inputs_4, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    for (i, asset_out) in simd_asset_outputs.iter().enumerate() {
        let out = &asset_out[0];
        println!(
            "\nSIMD by-assets Roofing Filter (asset {}): last 5 = {:?}",
            i,
            &out[out.len() - 5..]
        );
    }

    /////////////////////// SIMD by-options: 1 asset, 4 option sets ///////////////////////
    // With 80 bars of CLOSE data (min_data = max(ss, hp) + 1):
    //   (5, 20)  → min_data=21 → 60 output values
    //   (8, 30)  → min_data=31 → 50 output values
    //   (10, 40) → min_data=41 → 40 output values
    //   (10, 48) → min_data=49 → 32 output values
    let options_4 = [&[5.0f64, 20.0], &[8.0, 30.0], &[10.0, 40.0], &[10.0, 48.0]];

    let (simd_option_outputs, _) = match indicator_by_options::<4>(&inputs, &options_4, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    for (i, opts) in options_4.iter().enumerate() {
        let out = &simd_option_outputs[i][0];
        println!(
            "\nSIMD by-options Roofing Filter (ss={}, hp={}): last 5 = {:?}",
            opts[0],
            opts[1],
            &out[out.len() - 5..]
        );
    }
}

use tulip_rs::indicators::adaptivemsw::{indicator, indicator_by_assets, TIndicatorState};

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
    // Adaptive MESA Sine Wave — the true adaptive form of the MSW indicator.
    // The Homodyne Discriminator measures the dominant cycle period each bar;
    // a windowed DFT of that adaptive length extracts the instantaneous phase.
    // No user options are required.
    // min_data = 23 (HD warmup). First ~50 bars are transient while the DFT
    // window fills and the HD IIR converges.
    let options: [f64; 0] = [];
    let inputs = [CLOSE.as_slice()];

    // --- Full run with optional dc_period output enabled ---
    // outputs[0] = sine
    // outputs[1] = lead_sine
    // outputs[2] = dc_period (optional)
    let (outputs, _) = match indicator(&inputs, &options, Some(&[true])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Adaptive MSW — {} output bars", outputs[0].len());
    println!(
        "Sine      (last 10): {:?}",
        &outputs[0][outputs[0].len() - 10..]
    );
    println!(
        "Lead Sine (last 10): {:?}",
        &outputs[1][outputs[1].len() - 10..]
    );
    println!(
        "DC Period (last 10): {:?}",
        &outputs[2][outputs[2].len() - 10..]
    );
    println!("\nNote: early bars are transient while the HD IIR and DFT window converge.");

    // --- Streaming / continuation example ---
    let split = CLOSE.len() - 5;
    let inputs_partial = [&CLOSE[..split]];

    let (outputs_partial, mut state) = match indicator(&inputs_partial, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\nPartial run ({} bars):", split);
    println!(
        "  Sine last 5:      {:?}",
        &outputs_partial[0][outputs_partial[0].len() - 5..]
    );
    println!(
        "  Lead Sine last 5: {:?}",
        &outputs_partial[1][outputs_partial[1].len() - 5..]
    );

    let new_inputs = [&CLOSE[split..]];
    let continuation = match state.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Continuation — Sine:      {:?}", continuation[0]);
    println!("Continuation — Lead Sine: {:?}", continuation[1]);

    /////////////////////// SIMD by-assets: 4 assets, same options ///////////////////////
    let asset0: [&[f64]; 1] = [CLOSE.as_slice()];
    let close1: Vec<f64> = CLOSE.iter().map(|v| v * 1.1).collect();
    let asset1: [&[f64]; 1] = [close1.as_slice()];
    let close2: Vec<f64> = CLOSE.iter().map(|v| v * 0.9).collect();
    let asset2: [&[f64]; 1] = [close2.as_slice()];
    let close3: Vec<f64> = CLOSE
        .iter()
        .enumerate()
        .map(|(i, v)| v + i as f64 * 0.1)
        .collect();
    let asset3: [&[f64]; 1] = [close3.as_slice()];
    let inputs_4: [&[&[f64]; 1]; 4] = [&asset0, &asset1, &asset2, &asset3];

    let (simd_asset_outputs, _) = match indicator_by_assets::<4>(&inputs_4, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!(
        "\nSIMD by-assets Sine      last 5 (asset 0): {:?}",
        &simd_asset_outputs[0][0][simd_asset_outputs[0][0].len() - 5..]
    );
    println!(
        "SIMD by-assets Lead Sine last 5 (asset 0): {:?}",
        &simd_asset_outputs[0][1][simd_asset_outputs[0][1].len() - 5..]
    );
}

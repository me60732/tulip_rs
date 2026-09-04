use tulip_rs::indicator_types::{Indicator, IndicatorByOptions, TIndicatorState};
use tulip_rs::indicators::trendmode::TrendMode;

// 80 bars of close prices (trendmode needs min_data = 56)
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
    // Ehlers TrendMode — detects whether price is in Trend Mode (1.0) or Cycle Mode (0.0)
    // by comparing the CyberCycle amplitude to a decaying peak.
    //
    // Options: [alpha]  where alpha ∈ [0.0, 1.0).
    //   alpha = 0.0  → adaptive: alpha derived from the dominant cycle each bar.
    //   alpha > 0.0  → fixed. Ehlers' default is 0.07.
    // min_data = 56 (HD + CyberCycle warmup)
    let options = [0.07]; // Ehlers default fixed alpha

    let inputs = [CLOSE.as_slice()];

    // --- Full run with optional cycle and peak outputs ---
    // outputs[0] = trendmode  (1.0 = Trend, 0.0 = Cycle)
    // outputs[1] = cycle      (CyberCycle oscillator, optional)
    // outputs[2] = peak       (decaying amplitude peak, optional)
    let (outputs, _) = match TrendMode::indicator(&inputs, &options, Some(&[true, true])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("TrendMode — {} output bars", outputs[0].len());
    println!("TrendMode (all): {:?}", outputs[0]);
    println!("Cycle     (all): {:?}", outputs[1]);
    println!("Peak      (all): {:?}", outputs[2]);

    // --- Adaptive alpha example (alpha = 0.0) ---
    let options_adaptive = [0.0];
    let (outputs_adaptive, _) = match TrendMode::indicator(&inputs, &options_adaptive, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nTrendMode (adaptive alpha): {:?}", outputs_adaptive[0]);

    // --- Streaming / continuation example ---
    let split = CLOSE.len() - 5;
    let inputs_partial = [&CLOSE[..split]];

    let (outputs_partial, mut state) = match TrendMode::indicator(&inputs_partial, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!(
        "\nPartial run ({} bars, {} values):",
        split,
        outputs_partial[0].len()
    );
    println!("  TrendMode last: {:?}", outputs_partial[0].last());

    let new_inputs = [&CLOSE[split..]];
    let continuation = match state.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Continuation — TrendMode: {:?}", continuation[0]);
    println!("\n(The continuation values should match the tail of the full run above.)");

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

    let (simd_asset_outputs, _) =
        match TrendMode::indicator_by_assets::<4>(&inputs_4, &options, None) {
            Ok(result) => result,
            Err(e) => panic!("Error: {}", e),
        };
    for (i, outs) in simd_asset_outputs.iter().enumerate() {
        let len = outs[0].len();
        println!(
            "\nSIMD by-assets TrendMode last 5 (asset {}): {:?}",
            i,
            &outs[0][len - 5..]
        );
    }

    /////////////////////// SIMD by-options: 1 asset, 4 option sets ///////////////////////
    // alpha = 0.0 → adaptive mode: alpha is derived from the dominant cycle via the
    // embedded Hilbert Discriminator each bar (no fixed period).
    let options_4 = [&[0.0f64], &[0.05], &[0.07], &[0.10]];

    let (simd_option_outputs, _) =
        match TrendMode::indicator_by_options::<4>(&inputs, &options_4, None) {
            Ok(result) => result,
            Err(e) => panic!("Error: {}", e),
        };
    for (i, opts) in options_4.iter().enumerate() {
        let len = simd_option_outputs[i][0].len();
        let label = if opts[0] == 0.0 {
            format!("alpha={} (adaptive)", opts[0])
        } else {
            format!("alpha={}", opts[0])
        };
        println!(
            "\nSIMD by-options TrendMode last 5 ({}): {:?}",
            label,
            &simd_option_outputs[i][0][len - 5..]
        );
    }
}

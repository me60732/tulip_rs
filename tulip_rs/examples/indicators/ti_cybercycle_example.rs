use tulip_rs::indicator_types::{Indicator, IndicatorByOptions, TIndicatorState};
use tulip_rs::indicators::cybercycle::Cybercycle;

// 80 bars of close prices (cybercycle only needs min_data = 7)
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
    // Ehlers CyberCycle — a 2-pole high-pass IIR oscillator that isolates the dominant
    // market cycle. alpha controls the cutoff frequency.
    //
    // Options: [alpha]  where alpha is in (0.0, 1.0) exclusive.
    // Ehlers' default alpha = 0.07.
    // min_data = 7
    // Note: alpha = 0.0 is NOT valid here — use trendmode or ccfisher for adaptive alpha.
    let options = [0.07]; // Ehlers default alpha

    let inputs = [CLOSE.as_slice()];

    // --- Full run with optional trigger output ---
    // outputs[0] = cybercycle
    // outputs[1] = trigger (= Cycle[1], optional)
    let (outputs, _) = match Cybercycle::indicator(&inputs, &options, Some(&[true])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("CyberCycle — {} output bars", outputs[0].len());
    println!("CyberCycle (first 10): {:?}", &outputs[0][..10]);
    println!(
        "CyberCycle (last  5):  {:?}",
        &outputs[0][outputs[0].len() - 5..]
    );
    println!(
        "Trigger    (last  5):  {:?}",
        &outputs[1][outputs[1].len() - 5..]
    );

    // --- Streaming / continuation example ---
    let split = CLOSE.len() - 5;
    let inputs_partial = [&CLOSE[..split]];

    let (outputs_partial, mut state) = match Cybercycle::indicator(&inputs_partial, &options, None)
    {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!(
        "\nPartial run ({} bars, {} values):",
        split,
        outputs_partial[0].len()
    );
    println!(
        "  CyberCycle last 5: {:?}",
        &outputs_partial[0][outputs_partial[0].len() - 5..]
    );

    let new_inputs = [&CLOSE[split..]];
    let continuation = match state.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Continuation — CyberCycle: {:?}", continuation[0]);
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
        match Cybercycle::indicator_by_assets::<4>(&inputs_4, &options, None) {
            Ok(result) => result,
            Err(e) => panic!("Error: {}", e),
        };
    for (i, outs) in simd_asset_outputs.iter().enumerate() {
        let len = outs[0].len();
        println!(
            "\nSIMD by-assets CyberCycle last 5 (asset {}): {:?}",
            i,
            &outs[0][len - 5..]
        );
    }

    /////////////////////// SIMD by-options: 1 asset, 4 option sets ///////////////////////
    // alpha = 0.0 is NOT valid for cybercycle — validate_options rejects alpha <= 0.0.
    // Minimum valid alpha is just above 0.0; use 0.05 as the smallest test value.
    let options_4 = [&[0.05f64], &[0.07], &[0.10], &[0.15]];

    let (simd_option_outputs, _) =
        match Cybercycle::indicator_by_options::<4>(&inputs, &options_4, None) {
            Ok(result) => result,
            Err(e) => panic!("Error: {}", e),
        };
    for (i, opts) in options_4.iter().enumerate() {
        let len = simd_option_outputs[i][0].len();
        println!(
            "\nSIMD by-options CyberCycle last 5 (alpha={}): {:?}",
            opts[0],
            &simd_option_outputs[i][0][len - 5..]
        );
    }
}

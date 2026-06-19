use tulip_rs::indicators::ichimoku::{
    indicator, indicator_by_assets, indicator_by_options, TIndicatorState,
};

// 40 bars of OHLC prices
// With short_period=5, long_period=10: min_data = (10*2+1) + 10 = 31
const HIGH: [f64; 40] = [
    82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
    88.00, 87.87, 88.55, 89.60, 89.10, 88.44, 89.55, 90.22, 90.99, 90.75, 90.00, 88.88, 87.66,
    86.99, 86.22, 86.77, 87.95, 88.88, 90.07, 90.99, 91.68, 92.44, 91.95, 91.22, 90.33, 88.99,
    88.10,
];

const LOW: [f64; 40] = [
    81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
    87.17, 87.01, 87.44, 88.77, 88.22, 87.55, 88.66, 89.43, 90.22, 89.88, 89.11, 87.99, 86.77,
    86.11, 85.33, 85.88, 87.05, 87.99, 89.17, 90.10, 90.78, 91.55, 91.05, 90.33, 89.44, 88.10,
    87.22,
];

const CLOSE: [f64; 40] = [
    81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
    87.77, 87.29, 87.99, 89.17, 88.71, 88.00, 89.11, 89.87, 90.65, 90.31, 89.56, 88.44, 87.23,
    86.55, 85.77, 86.33, 87.50, 88.43, 89.62, 90.54, 91.23, 92.00, 91.50, 90.77, 89.90, 88.55,
    87.66,
];

fn main() {
    // Ichimoku Cloud (Ichimoku Kinkō Hyō)
    //
    // Options: [short_period, long_period]
    //   short_period — Tenkan-sen (Conversion Line) period, must be < long_period
    //   long_period  — Kijun-sen (Base Line) period
    // min_data = (long_period * 2 + 1) + long_period
    // Note: output lengths differ per component:
    //   conversion:    data_len - short_period + 1
    //   base:          data_len - long_period + 1
    //   leading_span_a/b: data_len - (long_period * 2) + 1
    //   lagging_span:  data_len (full close series, optional)
    let options = [5.0, 10.0]; // short_period = 5, long_period = 10

    let inputs = [HIGH.as_slice(), LOW.as_slice(), CLOSE.as_slice()];

    // --- Full run with optional lagging span (Chikou Span) ---
    // outputs[0] = conversion (Tenkan-sen)
    // outputs[1] = base       (Kijun-sen)
    // outputs[2] = leading_span_a (Senkou Span A, shifted +long_period bars forward)
    // outputs[3] = leading_span_b (Senkou Span B, shifted +long_period bars forward)
    // outputs[4] = lagging_span   (Chikou Span = close, shifted -long_period bars back, optional)
    let (outputs, _) = match indicator(&inputs, &options, Some(&[true])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!(
        "Ichimoku — input: {} bars, options: short={}, long={}",
        HIGH.len(),
        options[0],
        options[1]
    );
    println!(
        "Conversion    ({} values): {:?}",
        outputs[0].len(),
        outputs[0]
    );
    println!(
        "Base          ({} values): {:?}",
        outputs[1].len(),
        outputs[1]
    );
    println!(
        "Leading Span A({} values): {:?}",
        outputs[2].len(),
        outputs[2]
    );
    println!(
        "Leading Span B({} values): {:?}",
        outputs[3].len(),
        outputs[3]
    );
    println!(
        "Lagging Span  ({} values, first 5): {:?}",
        outputs[4].len(),
        &outputs[4][..5]
    );

    // --- Streaming / continuation example ---
    let split = 35; // first 35 bars (>= min_data of 31)
    let inputs_partial = [&HIGH[..split], &LOW[..split], &CLOSE[..split]];

    let (outputs_partial, mut state) = match indicator(&inputs_partial, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\nPartial run ({} bars):", split);
    println!("  Conversion last: {:?}", outputs_partial[0].last());
    println!("  Base last:       {:?}", outputs_partial[1].last());

    let new_inputs = [&HIGH[split..], &LOW[split..], &CLOSE[split..]];
    let continuation = match state.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Continuation — Conversion:     {:?}", continuation[0]);
    println!("Continuation — Base:           {:?}", continuation[1]);
    println!("Continuation — Leading Span A: {:?}", continuation[2]);
    println!("Continuation — Leading Span B: {:?}", continuation[3]);
    println!("\n(Continuation values should match the tail of the full run above.)");

    /////////////////////// SIMD by-assets: 4 assets, same options ///////////////////////
    let asset0: [&[f64]; 3] = [HIGH.as_slice(), LOW.as_slice(), CLOSE.as_slice()];
    let high1: Vec<f64> = HIGH.iter().map(|v| v * 1.05).collect();
    let low1: Vec<f64> = LOW.iter().map(|v| v * 1.05).collect();
    let close1: Vec<f64> = CLOSE.iter().map(|v| v * 1.05).collect();
    let asset1: [&[f64]; 3] = [high1.as_slice(), low1.as_slice(), close1.as_slice()];
    let high2: Vec<f64> = HIGH.iter().map(|v| v * 0.95).collect();
    let low2: Vec<f64> = LOW.iter().map(|v| v * 0.95).collect();
    let close2: Vec<f64> = CLOSE.iter().map(|v| v * 0.95).collect();
    let asset2: [&[f64]; 3] = [high2.as_slice(), low2.as_slice(), close2.as_slice()];
    let high3: Vec<f64> = HIGH.iter().map(|v| v * 1.10).collect();
    let low3: Vec<f64> = LOW.iter().map(|v| v * 1.10).collect();
    let close3: Vec<f64> = CLOSE.iter().map(|v| v * 1.10).collect();
    let asset3: [&[f64]; 3] = [high3.as_slice(), low3.as_slice(), close3.as_slice()];
    let inputs_4: [&[&[f64]; 3]; 4] = [&asset0, &asset1, &asset2, &asset3];

    let (simd_asset_outputs, _) = match indicator_by_assets::<4>(&inputs_4, &options, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    // Show last 3 of conversion and base for asset 0 only
    let conv0 = &simd_asset_outputs[0][0];
    let base0 = &simd_asset_outputs[0][1];
    println!(
        "\nSIMD by-assets Conversion last 3 (asset 0): {:?}",
        &conv0[conv0.len() - 3..]
    );
    println!(
        "SIMD by-assets Base       last 3 (asset 0): {:?}",
        &base0[base0.len() - 3..]
    );

    /////////////////////// SIMD by-options: 1 asset, 4 option sets ///////////////////////
    // Option sets where min_data = 3*long_period+1 <= 40 bars:
    //   [3.0,  7.0] → min_data = 22  → conversion has 38 values
    //   [5.0, 10.0] → min_data = 31  → conversion has 36 values
    //   [5.0, 12.0] → min_data = 37  → conversion has 36 values
    //   [7.0, 13.0] → min_data = 40  → conversion has 34 values
    let options_4 = [&[3.0, 7.0], &[5.0, 10.0], &[5.0, 12.0], &[7.0, 13.0]];

    let (simd_option_outputs, _) = match indicator_by_options::<4>(&inputs, &options_4, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    for (i, opts) in options_4.iter().enumerate() {
        let conv = &simd_option_outputs[i][0];
        println!(
            "\nSIMD by-options Conversion last 3 (short={}, long={}): {:?}",
            opts[0],
            opts[1],
            &conv[conv.len() - 3..]
        );
    }
}

use tulip_rs::indicators::vidya::{indicator, TIndicatorState};

fn main() {
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices
    let options = [2.0, 5.0, 0.2];

    let inputs = [close.as_slice()];

    let (result, _) = match indicator(&inputs, &options, Some(&[true, true, true, true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Full Vidya Line: {:?}", result[0]);
    println!("\nShort SMA Line: {:?}", result[1]);
    println!("\nLong SMA Line: {:?}", result[2]);
    println!("\nShort Stdev: {:?}", result[3]);
    println!("\nLong Stdev: {:?}", result[4]);

    let inputs2 = [&close[..close.len() - 5]];

    let (result2, mut state2) = match indicator(&inputs2, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\nPartial Vidya Line: {:?}", result2[0]);

    let inputs3 = [&close[close.len() - 5..]];
    let result = match state2.batch_indicator(&inputs3, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\nFinal Vidya Line: {:?}", result[0]);
}

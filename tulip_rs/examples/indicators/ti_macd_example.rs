use tulip_rs::indicators::macd::{indicator, TIndicatorState};

fn main() {
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ]; // Close prices
    let options = [2.0, 5.0, 9.0];

    let inputs = [close.as_slice()];

    //////////////////////////////////////////// MACD ////////////////////////////////////////////
    let (outputs, _) = match indicator(&inputs, &options, Some(&[true, true])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {:?}", e),
    };
    println!("Full MACD Line: {:?}", outputs[0]);
    println!("Full Signal Line: {:?}", outputs[1]);
    println!("Full Histrogram: {:?}", outputs[2]);
    println!("Full short_ema: {:?}", outputs[3]);
    println!("Full long_ema: {:?}", outputs[4]);

    ////////////////////////////// Partial MACD //////////////////////////////
    let inputs2 = [&close[0..close.len() - 1]];

    let (outputs2, mut state) = match indicator(&inputs2, &options, Some(&[true, true])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {:?}", e),
    };

    println!("\nPartial MACD Line: {:?}", outputs2[0]);
    println!("Partial Signal Line: {:?}", outputs2[1]);
    println!("Partial Histrogram: {:?}", outputs2[2]);

    let new_inputs = [&close[close.len() - 1..]];

    let final_outputs = match state.batch_indicator(&new_inputs, None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {:?}", e),
    };

    println!("\nFinal MACD Line: {:?}", final_outputs[0]);
    println!("Final Signal Line: {:?}", final_outputs[1]);
    println!("Final Histrogram: {:?}", final_outputs[2]);
}

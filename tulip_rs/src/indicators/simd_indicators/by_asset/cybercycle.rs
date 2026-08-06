use crate::common_simd::assets::validate_inputs;
use crate::indicators::cybercycle::{
    Cybercycle, Indicator, validate_options, IndicatorState, State, INPUTS, OPTIONS,
};
use crate::indicators::simd_indicators::cybercycle_simd::{SimdState, TSimdState, TState};
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::{IndicatorError, Warm};
use std::simd::Simd;

/// SIMD driver that advances the CyberCycle across `N` asset lanes per epoch.
///
/// Coefficients (`coef`, `d1`, `d2`) are embedded in each lane's scalar [`State`]
/// by [`State::init_state`] and gathered into the [`SimdState`] by
/// [`TSimdState::from_states`] — no external multiplier vectors are needed.
struct CycleDriver {
    want_trigger: bool,
}

impl Driver<State<Warm>> for CycleDriver {
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State<Warm>>,
        _options: Vec<Option<&()>>,
    ) {
        let len = inputs[0][0].len();

        // Coefficients are gathered from each lane's scalar State — no external mults needed.
        let mut simd_state = SimdState::from_states(&mut states);

        let real_ptrs = crate::extract_input_ptrs!(inputs, N, real_ptrs);
        let (cycle_ptrs, trigger_ptrs) =
            crate::extract_output_ptrs!(outputs, N, cycle_ptrs, trigger_ptrs);

        for i in 0..len {
            let real = crate::extract_simd_inputs_at_index!(i, N, real @ real_ptrs);
            // Safety: all ring buffers are full — guaranteed by State::init_state
            // called for every lane before PrimeMover dispatches.
            let cycle = simd_state.calc(real);
            crate::write_simd_at_indices!(N, i, cycle_ptrs => cycle);
            // trigger[i] = Cycle[i-1] = cycle_prev2 (updated inside calc_simd_unchecked)
            crate::store_simd_optional_outputs!(i, N,
                self.want_trigger, trigger_ptrs => simd_state.cycle_prev2
            );
        }

        simd_state.write_states(&mut states);
    }
}

/// Calculates the Ehlers CyberCycle for `N` assets simultaneously using SIMD.
///
/// Each asset's state is independently warmed up via [`State::init_state`] (consuming
/// bars 0–6, writing output index 0), then all assets are batched by the
/// [`PrimeMover`] scheduler starting at bar 7.
///
/// # Arguments
///
/// * `inputs`           — `N` asset input sets; `inputs[i]` is `[&[f64]; 1]` = `[real]`.
/// * `options`          — `[alpha; 1]`; same value applied to all N assets.
/// * `optional_outputs` — index `0` = `trigger`.
///
/// # Returns
///
/// `Ok((outputs, states))` where `outputs[i][0]` = cybercycle, `outputs[i][1]` = trigger
/// (empty unless requested), and `states[i]` is the final [`IndicatorState`] for asset `i`.
/// Returns `Err(NotEnoughData)` if any asset has fewer than 7 bars, or
/// `Err(InvalidOptions)` if `alpha` is not in `(0, 1)`.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS]; N],
    options: &[f64; OPTIONS],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>), IndicatorError> {
    validate_options(options)?;
    validate_inputs::<INPUTS>(inputs, Cybercycle::min_data(options))?;

    let alpha = options[0];
    let want_trigger = optional_outputs
        .and_then(|f| f.first().copied())
        .unwrap_or(false);

    let mut output_buffers = Vec::with_capacity(N);
    let mut road_train = PrimeMover::<N, State<Warm>>::new();

    for i in 0..N {
        let len = inputs[i][0].len();
        let capacity = Cybercycle::output_length(len, options);

        let mut cycle_line = crate::uninit_vec!(f64, capacity);
        let mut trigger_line: Vec<f64> = if want_trigger {
            crate::uninit_vec!(f64, capacity)
        } else {
            Vec::new()
        };

        // init_state seeds bars 0–5, processes bar 6, writes outputs[0],
        // and embeds coef/d1/d2 from alpha into the returned State.
        let state = State::init_state(inputs[i][0], alpha, &mut cycle_line, &mut trigger_line);

        // Slice outputs so the driver writes indices 1..capacity.
        let mut output_buffer = vec![cycle_line, trigger_line];
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());
        for j in 0..output_buffer.len() {
            unsafe {
                let buf = &mut output_buffer[j];
                let buf_len = buf.len();
                let start = if buf_len > 0 { 1 } else { 0 };
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    buf.as_mut_ptr().add(start),
                    buf_len.saturating_sub(start),
                ));
            }
        }

        road_train.add_asset(Asset::new(
            vec![inputs[i][0]],
            asset_outputs,
            i,
            // init_state consumed bars 0..6 inclusive; driver starts at bar 7 = min_data.
            Cybercycle::min_data(options),
            0,
            state,
            None,
        ));

        output_buffers.push(output_buffer);
    }

    let mut driver = CycleDriver { want_trigger };
    let final_states = road_train.drive(&mut driver);

    Ok((output_buffers, final_states))
}

/// Builds a `Vec<Simd<f64, N>>` from an array of `N` raw input pointers, reading `length` elements each.
#[macro_export]
macro_rules! create_simd_vec_from_inputs {
    ($input_ptr:ident, $width:expr, $length:expr) => {
        {
            let mut simd_vec: Vec<Simd<f64, $width>> = crate::uninit_vec!(Simd<f64, $width>, $length);
            for i in 0..$length {
                let mut values = [0.0; $width];
                for j in 0..$width {
                    unsafe {
                        values[j] = *$input_ptr[j].add(i);
                    }
                }
                unsafe {
                    *simd_vec.get_unchecked_mut(i) = Simd::from_array(values);
                }
            }
            simd_vec
        }
    };
}

/// Extracts raw `*const f64` pointer arrays (one per input field) from a nested `inputs` slice.
/// Returns a tuple of `[*const f64; N]` arrays.
#[macro_export]
macro_rules! extract_input_ptrs {
    ($inputs:expr, $width:expr, $($field_name:ident),*) => {
        {
            let mut _index = 0;
            (
                $(
                    {
                        let $field_name: [*const f64; $width] = std::array::from_fn(|j| unsafe {
                            $inputs.get_unchecked(j).get_unchecked(_index).as_ptr()
                        });
                        _index += 1;
                        $field_name
                    }
                ),*
            )
        }
    };
}

/// Extracts raw `*mut f64` pointer arrays (one per output field) from a nested `outputs` slice.
#[macro_export]
macro_rules! extract_output_ptrs {
    ($outputs:expr, $width:expr, $($field_name:ident),*) => {
        {
            let mut _index = 0;
            (
                $(
                    {
                        let $field_name: [*mut f64; $width] = std::array::from_fn(|j| unsafe {
                            $outputs.get_unchecked_mut(j).get_unchecked_mut(_index).as_mut_ptr()
                        });
                        _index += 1;
                        $field_name
                    }
                ),*
            )
        }
    };
}
/*#[macro_export]
macro_rules! extract_simd_inputs_at_index {
    ($index:expr, $width:expr, $($ptr_array:ident),*) => {
        {
            (
                $(
                    Simd::from_array(std::array::from_fn(|j| unsafe {
                        *$ptr_array[j].add($index)
                    }))
                ),*
            )
        }
    };
}*/

/// Gathers SIMD lanes from `N` input pointer arrays at per-lane indices.
#[macro_export]
macro_rules! extract_simd_inputs_at_index_array {
    ($indices:expr, $width:expr, $($var_name:ident @ $ptr_array:ident),*) => {
        {
            // Declare arrays for each named variable
            $(let mut $var_name = [0.0; $width];)*

            // Loop through all lanes, extracting from different input arrays at different indices
            for j in 0..$width {
                unsafe {
                    let index = $indices[j];
                    $($var_name[j] = *$ptr_array[j].add(index);)*
                }
            }

            // Convert to SIMD vectors and return as tuple
            ($(Simd::from_array($var_name)),*)
        }
    };
}

/// Same as `extract_simd_inputs_at_index_array!` but returns raw `f64` arrays instead of SIMD vectors.
#[macro_export]
macro_rules! extract_array_inputs_at_index_array {
    ($indices:expr, $width:expr, $($var_name:ident @ $ptr_array:ident),*) => {
        {
            // Declare arrays for each named variable
            $(let mut $var_name = [0.0; $width];)*

            // Loop through all lanes, extracting from different input arrays at different indices
            for j in 0..$width {
                unsafe {
                    let index = $indices[j];
                    $($var_name[j] = *$ptr_array[j].add(index);)*
                }
            }

            // Return raw arrays
            ($($var_name),*)
        }
    };
}

/// Gathers index `i` from each of `N` input pointer arrays into a SIMD vector, one per named field.
#[macro_export]
macro_rules! extract_simd_inputs_at_index {
    ($index:expr, $width:expr, $($var_name:ident @ $ptr_array:ident),*) => {
        {
            // Declare arrays for each named variable
            $(let mut $var_name = [0.0; $width];)*

            // Single loop through all assets, extracting from different input arrays
            for j in 0..$width {
                unsafe {
                    $($var_name[j] = *$ptr_array[j].add($index);)*
                }
            }

            // Convert to SIMD vectors and return as tuple
            ($(Simd::from_array($var_name)),*)
        }
    };
}

/// Broadcasts a single scalar value (lane 0) at index `i` into all `N` SIMD lanes.
#[macro_export]
macro_rules! extract_simd_inputs_at_index_splat {
    ($index:expr, $width:expr, $($var_name:ident @ $ptr_array:ident),*) => {
        {
            (
                $(
                    unsafe {
                        Simd::splat(*$ptr_array[0].add($index))
                    }
                ),*
            )
        }
    };
}

/// Same as `extract_simd_inputs_at_index!` but returns raw `f64` arrays.
#[macro_export]
macro_rules! extract_array_inputs_at_index {
    ($index:expr, $width:expr, $($var_name:ident @ $ptr_array:ident),*) => {
        {
            // Declare arrays for each named variable
            $(let mut $var_name = [0.0; $width];)*

            // Single loop through all assets, extracting from different input arrays
            for j in 0..$width {
                unsafe {
                    $($var_name[j] = *$ptr_array[j].add($index);)*
                }
            }

            // Return raw arrays
            ($($var_name),*)
        }
    };
}

/// Conditionally writes a SIMD value to `N` output pointer arrays at index `i` if the want flag is set.
#[macro_export]
macro_rules! store_simd_optional_outputs {
    ($index:expr, $width:expr, $($want_flag:expr, $ptr_array:ident => $simd_value:expr),*) => {
        $(
            if $want_flag {
                for j in 0..$width {
                    unsafe {
                        *$ptr_array[j].add($index) = $simd_value[j];
                    }
                }
            }
        )*
    };
}

/// Same as `store_simd_optional_outputs!` but multiplies by a correction factor before writing.
#[macro_export]
macro_rules! store_simd_optional_outputs_corrected {
    ($index:expr, $width:expr, $($want_flag:expr, $ptr_array:ident => corrected($simd_value:expr, $multiplier:expr)),*) => {
        $(
            if $want_flag {
                let values = ($simd_value * $multiplier);//.to_array();
                for j in 0..$width {
                    unsafe {
                        *$ptr_array[j].add($index) = values[j];
                    }
                }
            }
        )*
    };
}

/// Gathers multiple values at different offsets from a single set of `N` pointer arrays.
#[macro_export]
macro_rules! extract_simd_at_indices {
    ($width:expr, $ptr_array:ident, $($var_name:ident @ $index:expr),*) => {
        {
            // Declare arrays for each named variable
            $(let mut $var_name = [0.0; $width];)*

            // Single loop through all assets, extracting all needed values
            for j in 0..$width {
                unsafe {
                    let ptr = $ptr_array[j];
                    $($var_name[j] = *ptr.add($index);)*
                }
            }

            // Convert to SIMD vectors and return as tuple
            ($(Simd::from_array($var_name)),*)
        }
    };
}

/// Same as `extract_simd_at_indices!` but returns raw `f64` arrays.
#[macro_export]
macro_rules! extract_array_at_indices {
    ($width:expr, $ptr_array:ident, $($var_name:ident @ $index:expr),*) => {
        {
            // Declare arrays for each named variable
            $(let mut $var_name = [0.0; $width];)*

            // Single loop through all assets, extracting all needed values
            for j in 0..$width {
                unsafe {
                    let ptr = $ptr_array[j];
                    $($var_name[j] = *ptr.add($index);)*
                }
            }

            // Return raw arrays
            ($($var_name),*)
        }
    };
}

/// Gathers values at per-lane dynamic indices from a single pointer array set.
#[macro_export]
macro_rules! extract_simd_at_indices_array {
    ($width:expr, $ptr_array:ident, $($var_name:ident @ $indices_simd:expr),*) => {
        {
            // Declare arrays for each named variable
            $(let mut $var_name = [0.0; $width];)*

            // Loop through each lane (asset) first for better cache locality
            for j in 0..$width {
                unsafe {
                    let ptr = $ptr_array[j];
                    // Extract all needed values for this lane/asset at once
                    $(
                        //let indices_array = $indices_simd.to_array();
                        $var_name[j] = *ptr.add($indices_simd[j]);
                    )*
                }
            }

            // Convert to SIMD vectors and return as tuple
            ($(Simd::from_array($var_name)),*)
        }
    };
}

/// Writes one or more SIMD values to their respective output pointer arrays at index `i`.
#[macro_export]
macro_rules! write_simd_at_indices {
    ($width:expr, $index:expr, $($ptr_array:ident => $simd_value:expr),*) => {
        unsafe {
            $(
                for j in 0..$width {
                    *$ptr_array[j].add($index) = $simd_value[j];
                }
            )*
        }
    };
}

/// Gathers the current value (offset 0) from each of `N` input pointer arrays into a SIMD vector.
#[macro_export]
macro_rules! extract_simd_from_ptrs {
    // Usage:
    // let (a, b, c, d) = crate::extract_simd_from_ptrs!(N, a_ptrs, b_ptrs, c_ptrs, d_ptrs);
    ($width:expr, $($var_name:ident @ $ptr_array:ident),+) => {
        {
            $(
                let mut $var_name = [0.0; $width];
            )+

            for j in 0..$width {
                unsafe {
                    $(
                        $var_name[j] = *$ptr_array[j];
                    )+
                }
            }

            ($(Simd::from_array($var_name)),+)
        }
    };
}

// ════════════════════════════════════════════════════════════════════════════════
// simd_state_from_state! / simd_state_write! / simd_state_impl!
// boilerplate-reducing macros for TSimdState::from_states / write_states
// ════════════════════════════════════════════════════════════════════════════════

/// Generates only the `from_states()` method inside a `TSimdState` trait impl block.
///
/// Three field categories, all optional:
/// - `sub`  — sub-states gathered as `&mut` refs, constructed via `SubType::from_states(&mut refs)`
/// - `f64`  — scalar fields packed into `[f64; N]` arrays then wrapped with `Simd::from_array`
/// - `buf`  — SIMD ring-buffer fields gathered as immutable refs (`&state.field`) and
///            constructed via the named method: `(field: SimdBufType, constructor_method)`
///
/// Field names must match between `SimdState` and `Self::ScalarState`. Either list may be
/// empty. Trailing commas are allowed. Omitting `buf:` is backwards-compatible.
///
/// # Usage — sub + f64 only
/// ```
/// simd_state_from_state!(
///     sub: [(dx_state: DxSimdState<N>)],
///     f64: [adx, prev_close]
/// );
/// ```
///
/// # Usage — with ring buffers
/// ```
/// simd_state_from_state!(
///     sub: [(adx_state: AdxSimdState<N>)],
///     scalar: [],
///     buf: [(buffer: SimdBuffer<N>, from_f64_buffers)]
/// );
/// ```
#[macro_export]
macro_rules! simd_state_from_state {
    // Full form: sub + scalar + buf + mask
    (
        sub: [ $( ($sf_field:ident : $SubType:path) ),* $(,)? ],
        scalar: [ $( $ff_field:ident ),* $(,)? ],
        buf: [ $( ($bf_field:ident : $BufType:path, $constructor:ident) ),* $(,)? ],
        mask: [ $( $mf_field:ident ),* $(,)? ]
    ) => {
        fn from_states(states: &mut [&mut Self::ScalarState]) -> Self {
            $( let mut $sf_field = Vec::with_capacity(N); )*
            $( let mut $ff_field = [Default::default(); N]; )*
            $( let mut $bf_field = Vec::with_capacity(N); )*
            $( let mut $mf_field = [false; N]; )*

            // Single shared loop: gather sub-state refs, scalar values, immutable buffer refs,
            // and mask values. _i suppresses the unused-variable warning when scalar/mask lists are empty.
            for (_i, state) in states.iter_mut().enumerate() {
                $( $sf_field.push(&mut state.$sf_field); )*
                $( $ff_field[_i] = state.$ff_field; )*
                $( $bf_field.push(&state.$bf_field); )*
                $( $mf_field[_i] = state.$mf_field; )*
            }

            $( let $sf_field = <$SubType>::from_states(&mut $sf_field); )*
            $( let $bf_field = <$BufType>::$constructor($bf_field); )*

            Self {
                $( $sf_field, )*
                $( $ff_field: Simd::from_array($ff_field), )*
                $( $bf_field, )*
                $( $mf_field: Mask::from_array($mf_field), )*
            }
        }
    };
    // Short form: sub + scalar + buf (no mask)
    (
        sub: [ $( ($sf_field:ident : $SubType:path) ),* $(,)? ],
        scalar: [ $( $ff_field:ident ),* $(,)? ],
        buf: [ $( ($bf_field:ident : $BufType:path, $constructor:ident) ),* $(,)? ]
    ) => {
        crate::simd_state_from_state!(
            sub: [ $( ($sf_field : $SubType) ),* ],
            scalar: [ $( $ff_field ),* ],
            buf: [ $( ($bf_field : $BufType, $constructor) ),* ],
            mask: []
        );
    };
    // Short form: sub + scalar only (no buf, no mask)
    (
        sub: [ $( ($sf_field:ident : $SubType:path) ),* $(,)? ],
        scalar: [ $( $ff_field:ident ),* $(,)? ]
    ) => {
        crate::simd_state_from_state!(
            sub: [ $( ($sf_field : $SubType) ),* ],
            scalar: [ $( $ff_field ),* ],
            buf: [],
            mask: []
        );
    };
}

/// Generates only the `write_states()` method inside a `TSimdState` trait impl block.
///
/// Use alongside [`simd_state_from_state!`]. List only the fields that should be scattered
/// back — omit read-only fields such as cached multipliers.
///
/// Buffer fields are scattered via `self.field.to_f64_buffers()` in a separate drain pass
/// before the main sub-state / f64 loop, so `states` can be safely re-borrowed. The
/// constructor name in `buf` entries is accepted for syntax consistency with
/// [`simd_state_from_state!`] but is not used here.
///
/// # Usage — sub + f64 only
/// ```
/// simd_state_write!(
///     sub: [(dx_state: DxSimdState<N>)],
///     f64: [adx]
/// );
/// ```
///
/// # Usage — with ring buffers
/// ```
/// simd_state_write!(
///     sub: [(adx_state: AdxSimdState<N>)],
///     scalar: [],
///     buf: [(buffer: SimdBuffer<N>, from_f64_buffers)]
/// );
/// ```
#[macro_export]
macro_rules! simd_state_write {
    // Full form: sub + scalar + buf + mask
    (
        sub: [ $( ($sf_field:ident : $SubType:path) ),* $(,)? ],
        scalar: [ $( $ff_field:ident ),* $(,)? ],
        buf: [ $( ($bf_field:ident : $BufType:path, $constructor:ident) ),* $(,)? ],
        mask: [ $( $mf_field:ident ),* $(,)? ]
    ) => {
        fn write_states(&self, states: &mut [&mut Self::ScalarState]) {
            $( let mut $sf_field = Vec::with_capacity(N); )*
            $( let $ff_field = self.$ff_field.to_array(); )*
            $( let mut $bf_field = self.$bf_field.to_f64_buffers(); )*
            $( let $mf_field = self.$mf_field.to_array(); )*

            // Scatter each buffer field back via drain before the main loop.
            // Each drain fully consumes its iterator, releasing the borrow on states
            // so it can be re-borrowed in the loop below.
            $(
                for (buf, state) in $bf_field.drain(..).zip(states.iter_mut()) {
                    state.$bf_field = buf;
                }
            )*

            // Main loop: collect sub-state refs and scatter scalar and mask values.
            for (_i, state) in states.iter_mut().enumerate() {
                $( $sf_field.push(&mut state.$sf_field); )*
                $( state.$ff_field = $ff_field[_i]; )*
                $( state.$mf_field = $mf_field[_i]; )*
            }

            $( self.$sf_field.write_states(&mut $sf_field); )*
        }
    };
    // Short form: sub + scalar + buf (no mask)
    (
        sub: [ $( ($sf_field:ident : $SubType:path) ),* $(,)? ],
        scalar: [ $( $ff_field:ident ),* $(,)? ],
        buf: [ $( ($bf_field:ident : $BufType:path, $constructor:ident) ),* $(,)? ]
    ) => {
        crate::simd_state_write!(
            sub: [ $( ($sf_field : $SubType) ),* ],
            scalar: [ $( $ff_field ),* ],
            buf: [ $( ($bf_field : $BufType, $constructor) ),* ],
            mask: []
        );
    };
    // Short form: sub + scalar only (no buf, no mask)
    (
        sub: [ $( ($sf_field:ident : $SubType:path) ),* $(,)? ],
        scalar: [ $( $ff_field:ident ),* $(,)? ]
    ) => {
        crate::simd_state_write!(
            sub: [ $( ($sf_field : $SubType) ),* ],
            scalar: [ $( $ff_field ),* ],
            buf: [],
            mask: []
        );
    };
}

/// Generates both `from_states()` and `write_states()` with identical field lists, for the
/// common case where every field round-trips to and from the scalar state.
///
/// Equivalent to calling [`simd_state_from_state!`] and [`simd_state_write!`] with the same
/// arguments. When some fields should not be written back (e.g. cached multipliers), use
/// those two macros separately instead.
///
/// # Usage — sub + f64 only
/// ```
/// impl<const N: usize> TSimdState for SimdState<N> {
///     type ScalarState = State;
///     simd_state_impl!(
///         sub: [(dx_state: DxSimdState<N>)],
///         scalar: [adx, prev_close]
///     );
/// }
/// ```
///
/// # Usage — with ring buffers
/// ```
/// simd_state_impl!(
///     sub: [(adx_state: AdxSimdState<N>)],
///     scalar: [],
///     buf: [(buffer: SimdBuffer<N>, from_f64_buffers)]
/// );
/// ```
#[macro_export]
macro_rules! simd_state_impl {
    // Full form: sub + scalar + buf + mask
    (
        sub: [ $( ($sf_field:ident : $SubType:path) ),* $(,)? ],
        scalar: [ $( $ff_field:ident ),* $(,)? ],
        buf: [ $( ($bf_field:ident : $BufType:path, $constructor:ident) ),* $(,)? ],
        mask: [ $( $mf_field:ident ),* $(,)? ]
    ) => {
        crate::simd_state_from_state!(
            sub: [ $( ($sf_field : $SubType) ),* ],
            scalar: [ $( $ff_field ),* ],
            buf: [ $( ($bf_field : $BufType, $constructor) ),* ],
            mask: [ $( $mf_field ),* ]
        );
        crate::simd_state_write!(
            sub: [ $( ($sf_field : $SubType) ),* ],
            scalar: [ $( $ff_field ),* ],
            buf: [ $( ($bf_field : $BufType, $constructor) ),* ],
            mask: [ $( $mf_field ),* ]
        );
    };
    // Short form: sub + scalar + buf (no mask)
    (
        sub: [ $( ($sf_field:ident : $SubType:path) ),* $(,)? ],
        scalar: [ $( $ff_field:ident ),* $(,)? ],
        buf: [ $( ($bf_field:ident : $BufType:path, $constructor:ident) ),* $(,)? ]
    ) => {
        crate::simd_state_from_state!(
            sub: [ $( ($sf_field : $SubType) ),* ],
            scalar: [ $( $ff_field ),* ],
            buf: [ $( ($bf_field : $BufType, $constructor) ),* ]
        );
        crate::simd_state_write!(
            sub: [ $( ($sf_field : $SubType) ),* ],
            scalar: [ $( $ff_field ),* ],
            buf: [ $( ($bf_field : $BufType, $constructor) ),* ]
        );
    };
    // Short form: sub + scalar only (no buf, no mask)
    (
        sub: [ $( ($sf_field:ident : $SubType:path) ),* $(,)? ],
        scalar: [ $( $ff_field:ident ),* $(,)? ]
    ) => {
        crate::simd_state_from_state!(
            sub: [ $( ($sf_field : $SubType) ),* ],
            scalar: [ $( $ff_field ),* ]
        );
        crate::simd_state_write!(
            sub: [ $( ($sf_field : $SubType) ),* ],
            scalar: [ $( $ff_field ),* ]
        );
    };
}

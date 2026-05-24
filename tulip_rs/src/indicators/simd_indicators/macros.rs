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

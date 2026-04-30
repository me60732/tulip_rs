#[macro_export]
macro_rules! store_optional_outputs {
    // minus offset
    ($index:expr, $($want_flag:expr, $output:ident[-$offset:expr] => $value:expr),*) => {
        $(
            if $want_flag {
                unsafe {
                    *$output.get_unchecked_mut($index - $offset) = $value;
                }
            }
        )*
    };
    //add offset
    ($index:expr, $($want_flag:expr, $output:ident[+$offset:expr] => $value:expr),*) => {
        $(
            if $want_flag {
                unsafe {
                    *$output.get_unchecked_mut($index + $offset) = $value;
                }
            }
        )*
    };
    //no offset
    ($index:expr, $($want_flag:expr, $output:ident => $value:expr),*) => {
        $(
            if $want_flag {
                unsafe {
                    *$output.get_unchecked_mut($index) = $value;
                }
            }
        )*
    };
}
#[macro_export]
macro_rules! store_optional_outputs_corrected {
    // minus offset
    // minus offset with correction
       ($index:expr, $($want_flag:expr, $output:ident[-$offset:expr] => corrected($value:expr, $multiplier:expr)),*) => {
           $(
               if $want_flag {
                   unsafe {
                       *$output.get_unchecked_mut($index - $offset) = $value * $multiplier;
                   }
               }
           )*
       };
       //add offset with correction
       ($index:expr, $($want_flag:expr, $output:ident[+$offset:expr] => corrected($value:expr, $multiplier:expr)),*) => {
           $(
               if $want_flag {
                   unsafe {
                       *$output.get_unchecked_mut($index + $offset) = $value * $multiplier;
                   }
               }
           )*
       };
       //no offset with correction
       ($index:expr, $($want_flag:expr, $output:ident => corrected($value:expr, $multiplier:expr)),*) => {
           $(
               if $want_flag {
                   unsafe {
                       *$output.get_unchecked_mut($index) = $value * $multiplier;
                   }
               }
           )*
       };
}
#[macro_export]
macro_rules! init_store_optional_outputs {
    //init calc offset
    ($index:expr, $data_len:expr, $($output:ident => $value:expr),*) => {
        $(
            if $output.len() > 0 {
                let offset = $data_len - $output.len();
                if $index >= offset {
                    $output[$index - offset] = $value;
                }
            }
        )*
    };
    //init no offset
    /*($index:expr, $data_len:expr, $($output:ident[no_offset] => $value:expr),*) => {
        $(
            if $output.len() > 0 {
                if $index < $output.len() {
                    $output[$index] = $value;
                }
            }
        )*
    };*/
}
#[macro_export]
macro_rules! store_optional_outputs_safe {
    // minus offset
    ($index:expr, $($want_flag:expr, $output:ident[-$offset:expr] => $value:expr),*) => {
        $(
            if $want_flag {
                $output[$index - $offset] = $value;
            }
        )*
    };
    //add offset
    ($index:expr, $($want_flag:expr, $output:ident[+$offset:expr] => $value:expr),*) => {
        $(
            if $want_flag {
                $output[$index + $offset] = $value;
            }
        )*
    };
    //no offset
    ($index:expr, $($want_flag:expr, $output:ident => $value:expr),*) => {
        $(
            if $want_flag {
                $output[$index] = $value;
            }
        )*
    };
    //init calc offset
    ($index:expr, $data_len:expr, $($output:ident => $value:expr),*) => {
        $(
            if $output.len() > 0 {
                let offset = $data_len - $output.len();
                if $index >= offset {
                    $output[$index - offset] = $value;
                }
            }
        )*
    };
    //init no offset
    ($index:expr, $data_len:expr, $($output:ident[no_offset] => $value:expr),*) => {
        $(
            if $output.len() > 0 {
                if $index < $output.len() {
                    $output[$index] = $value;
                }
            }
        )*
    };
}

#[macro_export]
macro_rules! init_optional_outputs {
    ($optional_outputs:expr, $defaults:expr, $($name:ident : $capacity:expr),*) => {
        {
            let optional_flags = $optional_outputs.unwrap_or($defaults);
            let mut _idx = 0;
            $(
                let $name = if optional_flags.get(_idx).copied().unwrap_or(false) {
                    vec![0.0; $capacity]
                } else {
                    Vec::new()
                };
                _idx += 1;
            )*
            ($($name),*)
        }
    };
}
#[macro_export]
macro_rules! init_optional_outputs_eff {
    ($optional_outputs:expr, $defaults:expr, $($name:ident : $capacity:expr),*) => {
        {
            let optional_flags = $optional_outputs.unwrap_or($defaults);
            let mut _idx = 0;
            $(
                let $name = if optional_flags.get(_idx).copied().unwrap_or(false) {
                    crate::uninit_vec!(f64, $capacity)
                } else {
                    Vec::new()
                };
                _idx += 1;
            )*
            ($($name),*)
        }
    };
}
#[macro_export]
macro_rules! calc_want_flags {
    ($($output:ident),*) => {
        {
            let has_optional = $($output.len() > 0)||*;
            (has_optional, $($output.len() > 0),*)
        }
    };
}

#[macro_export]
macro_rules! calc_output_offsets {
    ($data_len:expr, $($output:ident),*) => {
        {
            (
                $(
                    if $output.len() == 0 {
                        0
                    } else {
                        $data_len.saturating_sub($output.len())
                    }
                ),*
            )
        }
    };
}
/*#[macro_export]
macro_rules! calc_output_offsets_from_shortest {
    ($shortest_len:expr, $($output:ident),*) => {
        {
            (
                $(
                    if $output.len() == 0 {
                        0
                    } else {
                        $output.len() - $shortest_len
                    }
                ),*
            )
        }
    };
}*/
#[macro_export]
macro_rules! uninit_vec {
    ($type:ty, $len:expr) => {{
        let mut v: Vec<$type> = Vec::with_capacity($len);
        unsafe {
            v.set_len($len);
        }
        v
    }};
}

#[macro_export]
macro_rules! init_indicator_outputs {
    // Pattern with optional outputs specified
    ($optional_outputs:expr, $defaults:expr, $num_optional:expr, $($name:ident : $capacity:expr),*) => {
        {
            let optional_flags = $optional_outputs.unwrap_or($defaults);
            let mut indicator_outputs = Vec::new();
            let mut _idx = 0;
            let mut _optional_idx = 0;

            // Count total outputs
            let total_outputs = 0 $(+ { let _ = stringify!($name); 1 })*;
            let required_outputs = total_outputs - $num_optional;

            $(
                let $name = if _idx < required_outputs {
                    // Required output - always create
                    crate::uninit_vec!(f64, $capacity)
                } else {
                    // Optional output - check flags
                    if optional_flags.get(_optional_idx).copied().unwrap_or(false) {
                        let vec = crate::uninit_vec!(f64, $capacity);
                        _optional_idx += 1;
                        vec
                    } else {
                        _optional_idx += 1;
                        Vec::new()
                    }
                };
                indicator_outputs.push($name);
                _idx += 1;
            )*
            indicator_outputs
        }
    };

    // Pattern without optional outputs (all outputs are required)
    ($($name:ident : $capacity:expr),*) => {
        {
            let mut indicator_outputs = Vec::new();
            $(
                let $name = crate::uninit_vec!(f64, $capacity);
                indicator_outputs.push($name);
            )*
            indicator_outputs
        }
    };
}

#[macro_export]
macro_rules! slice_outputs_start {
    ($shortest_length:expr, $($output:ident),*) => {
        {
            (
                $(
                    if $output.len() > 0 && $output.len() >= $shortest_length {
                        $output.len().saturating_sub($shortest_length)
                    } else {
                        0
                    }
                ),*
            )
        }
    };
}


use std::collections::HashMap;

/// Carries a single value which may be interpreted as on/off, true/false, 1/0, etc. Logically
/// equivalent to a one-bit-wide Bus.
pub struct Wire {}

/// A (logical) collection of wires, carrying related signals, as in the bits of a value treated as binary data.
/// Typical width is the "word size" of the simulated machine, or less.
pub struct Bus {
    bits: usize
}

#[derive(Debug, PartialEq)]
pub enum ConnectionWidth {
    /// A connection carrying a single signal.
    Wire,

    /// A connection carrying multiple parallel signals, typically at least 2 and most often the
    /// word size of the simulated machine.
    Bus { width: u32 },
}

/// Declare the connections for a certain type of component. When an instance is added to an
/// Assembly, all connected inputs and outputs must match for width.
/// Any unconnected input may be assumed to be zero; any unconnected output is
pub trait Component {
    fn inputs(&self) -> HashMap<String, ConnectionWidth>;
    fn outputs(&self) -> HashMap<String, ConnectionWidth>;
}

/// Declare a component type from a concise description of its input and output connections.
///
/// # Example
/// ```
/// # use simulator::component;
/// component! {
///     pub struct Nand {
///         a: in,
///         b: in,
///         out: out,
///     }
/// }
/// ```
/// Generates a unit struct and a `Component` impl mapping each field name to a `Wire` connection.
#[macro_export]
macro_rules! component {
    // Entry point: strip visibility and delegate to accumulator
    ($vis:vis struct $name:ident { $($rest:tt)* }) => {
        $crate::component! {@collect $vis $name [] [] $($rest)*}
    };

    // `field: in,`
    (@collect $vis:vis $name:ident [$($ins:ident)*] [$($outs:ident)*]
        $field:ident : in , $($rest:tt)*) => {
        $crate::component! {@collect $vis $name [$($ins)* $field] [$($outs)*] $($rest)*}
    };
    // // `field: in`  (last field, no trailing comma)
    // (@collect $vis:vis $name:ident [$($ins:ident)*] [$($outs:ident)*]
    //     $field:ident : in) => {
    //     $crate::component! {@collect $vis $name [$($ins)* $field] [$($outs)*]}
    // };

    // `field: out,`
    (@collect $vis:vis $name:ident [$($ins:ident)*] [$($outs:ident)*]
        $field:ident : out , $($rest:tt)*) => {
        $crate::component! {@collect $vis $name [$($ins)*] [$($outs)* $field] $($rest)*}
    };
    // // `field: out`  (last field, no trailing comma)
    // (@collect $vis:vis $name:ident [$($ins:ident)*] [$($outs:ident)*]
    //     $field:ident : out) => {
    //     $crate::component! {@collect $vis $name [$($ins)*] [$($outs)* $field]}
    // };

    // Terminal: emit the struct and impl
    (@collect $vis:vis $name:ident [$($in_field:ident)*] [$($out_field:ident)*]) => {
        $vis struct $name;
        impl $crate::Component for $name {
            fn inputs(&self) -> ::std::collections::HashMap<::std::string::String, $crate::ConnectionWidth> {
                ::std::collections::HashMap::from([
                    $( (::std::string::String::from(stringify!($in_field)), $crate::ConnectionWidth::Wire) ),*
                ])
            }
            fn outputs(&self) -> ::std::collections::HashMap<::std::string::String, $crate::ConnectionWidth> {
                ::std::collections::HashMap::from([
                    $( (::std::string::String::from(stringify!($out_field)), $crate::ConnectionWidth::Wire) ),*
                ])
            }
        }
    };
}

pub mod bool;
pub mod declare;
pub mod eval;
pub mod nat;

pub use eval::eval;

pub use declare::*;

pub use simulator_derive::Reflect;

/// Show the connections of a chip after one level of expansion.
///
/// Each line is `source -> sink`. Chip inputs are sources; chip outputs are sinks.
/// Sub-component ports are labelled `{typename}{index}.{port}`.
///
/// ```ignore
/// let chip = And { a: Input::new(), b: Input::new(), out: Output::new() };
/// assert_eq!(print_graph(&chip), "And:\na -> nand0.a\nb -> nand0.b\nnand0.out -> not1.a\nnot1.out -> out");
/// ```
pub fn print_graph<C>(chip: &C) -> String
where
    C: Component + Reflect,
    C::Target: Component<Target = C::Target> + Reflect,
{
    use std::collections::HashMap;
    use std::rc::Rc;

    let intf = chip.reflect();
    let subs = match chip.expand() {
        None    => return String::new(),
        Some(s) => s,
    };

    let wire_id = |b: &BusRef| Rc::as_ptr(&b.id) as usize;

    // wire_id -> Vec<(label, is_sink)>
    let mut wires: HashMap<usize, Vec<(String, bool)>> = HashMap::new();

    for (port, busref) in &intf.inputs {
        wires.entry(wire_id(busref)).or_default().push((port.clone(), false));
    }
    for (port, busref) in &intf.outputs {
        wires.entry(wire_id(busref)).or_default().push((port.clone(), true));
    }
    for (i, sub) in subs.iter().enumerate() {
        let sub_intf = sub.reflect();
        let label = format!("{}{}", sub.name().to_lowercase(), i);
        for (port, busref) in &sub_intf.inputs {
            wires.entry(wire_id(busref)).or_default().push((format!("{}.{}", label, port), true));
        }
        for (port, busref) in &sub_intf.outputs {
            wires.entry(wire_id(busref)).or_default().push((format!("{}.{}", label, port), false));
        }
    }

    let mut lines: Vec<String> = wires.values()
        .flat_map(|endpoints| {
            let sources: Vec<_> = endpoints.iter().filter(|(_, s)| !s).map(|(n, _)| n.clone()).collect();
            let sinks:   Vec<_> = endpoints.iter().filter(|(_, s)| *s).map(|(n, _)| n.clone()).collect();
            sources.iter().flat_map(|src| {
                sinks.iter().map(|sink| format!("{} -> {}", src, sink)).collect::<Vec<_>>()
            }).collect::<Vec<_>>()
        })
        .collect();
    lines.sort();
    format!("{}:\n{}", chip.name(), lines.join("\n"))
}
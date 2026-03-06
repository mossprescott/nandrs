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
fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek(), bi.peek()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, _)    => return std::cmp::Ordering::Less,
            (_, None)    => return std::cmp::Ordering::Greater,
            (Some(ac), Some(bc)) if ac.is_ascii_digit() && bc.is_ascii_digit() => {
                let an: u64 = ai.by_ref().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap();
                let bn: u64 = bi.by_ref().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap();
                let ord = an.cmp(&bn);
                if ord != std::cmp::Ordering::Equal { return ord; }
            }
            (Some(ac), Some(bc)) => {
                let ord = ac.cmp(bc);
                if ord != std::cmp::Ordering::Equal { return ord; }
                ai.next(); bi.next();
            }
        }
    }
}

pub fn print_graph<C>(chip: &C) -> String
where
    C: Component + Reflect,
    C::Target: Component<Target = C::Target> + Reflect,
{
    use std::collections::{BTreeSet, HashMap};
    use std::rc::Rc;

    let intf = chip.reflect();
    let subs = match chip.expand() {
        None    => return String::new(),
        Some(s) => s,
    };

    let wire_id = |b: &BusRef| Rc::as_ptr(&b.id) as usize;

    // wire_id -> Vec<(label, is_sink, offset, width)>
    let mut wires: HashMap<usize, Vec<(String, bool, usize, usize)>> = HashMap::new();

    for (port, busref) in &intf.inputs {
        wires.entry(wire_id(busref)).or_default()
            .push((port.clone(), false, busref.offset, busref.width));
    }
    for (port, busref) in &intf.outputs {
        wires.entry(wire_id(busref)).or_default()
            .push((port.clone(), true, busref.offset, busref.width));
    }
    for (i, sub) in subs.iter().enumerate() {
        let sub_intf = sub.reflect();
        let label = format!("{}{}", sub.name().to_lowercase(), i);
        for (port, busref) in &sub_intf.inputs {
            wires.entry(wire_id(busref)).or_default()
                .push((format!("{}.{}", label, port), true, busref.offset, busref.width));
        }
        for (port, busref) in &sub_intf.outputs {
            wires.entry(wire_id(busref)).or_default()
                .push((format!("{}.{}", label, port), false, busref.offset, busref.width));
        }
    }

    let bit_label = |name: &str, width: usize, bit: usize| -> String {
        if width == 1 { name.to_string() } else { format!("{}[{}]", name, bit) }
    };

    let mut lines: Vec<String> = wires.values()
        .flat_map(|endpoints| {
            // Collect every individual bit offset referenced by any endpoint in this wire group.
            let bits: BTreeSet<usize> = endpoints.iter()
                .flat_map(|&(_, _, off, w)| off..off + w)
                .collect();
            let mut result = vec![];
            for bit in bits {
                let sources: Vec<_> = endpoints.iter()
                    .filter(|&&(_, is_sink, off, w)| !is_sink && off <= bit && bit < off + w)
                    .map(|(name, _, _, w)| bit_label(name, *w, bit))
                    .collect();
                let sinks: Vec<_> = endpoints.iter()
                    .filter(|&&(_, is_sink, off, w)| is_sink && off <= bit && bit < off + w)
                    .map(|(name, _, _, w)| bit_label(name, *w, bit))
                    .collect();
                for src in &sources {
                    for sink in &sinks {
                        result.push(format!("{} -> {}", src, sink));
                    }
                }
            }
            result
        })
        .collect();

    // Chip-input lines (source has no '.') sort before sub-component-output lines.
    lines.sort_by(|a, b| {
        let a_sub = a.split(" -> ").next().unwrap_or("").contains('.');
        let b_sub = b.split(" -> ").next().unwrap_or("").contains('.');
        match (a_sub, b_sub) {
            (false, true)  => std::cmp::Ordering::Less,
            (true,  false) => std::cmp::Ordering::Greater,
            _              => natural_cmp(a, b),
        }
    });
    format!("{}:\n{}", chip.name(), lines.join("\n"))
}
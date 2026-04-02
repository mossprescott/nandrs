pub mod bool;
pub mod component;
pub mod declare;
pub mod device;
pub mod eval;
pub mod nat;
pub mod simulate;
pub mod word;

#[cfg(test)]
mod tests;

pub use eval::eval;

pub use declare::{
    Chip, Component, IC, Input, Input1, Input16, InputBus, Interface, Output, Output16, OutputBus,
    Reflect, fixed,
};

pub use paste;
pub use simulator_derive::{Chip, Component, Reflect};

/// Either terminal results already in `T`, or an `IC<S>` whose components need further flattening.
///
/// Note: there's a middle case where a result `IC` already has components in the target type, but
/// there seems to be no harm in using `Continue` for that case, at least for current purposes.
///
/// TODO: is `IC<S>` sensible here? Probably never used at this point, but it's what `expand_t`
/// gives you, and in theory we would want to capture some info from it. On the other hand, `Done`
/// *usually* contains a single component that isn't an `IC` in any sense.
pub enum Flat<S, T> {
    /// Intermediate result, still in the source type; components need further flattening.
    Continue(IC<S>),
    /// Fully resolved target values; no further flattening needed.
    Done(Vec<T>),
}

/// Flatten to an arbitrary result type using a folder hlist. For each coproduct variant, the folder
/// produces either a value already in `T`, or an `IC<S>` whose components recurse through `go`.
///
/// Note: the types here are stronger; if it terminates, everything is reduced. To make that work,
/// every term in `S` has a well-defined expansion in that type. However, there's no structural
/// guarantee that expansion actually makes progress. We know that it does, because in practice each
/// `expand_t`'s type is smaller than `S` — it's at least missing the component being expanded.
/// Strictly speaking, nothing stops you from expanding A → B, then B → A, etc. Probably not worth
/// trying to bake that kind of guarantee into these types.
pub fn flatten_g<C, S, Idx, T, F>(chip: C, label: &str, folder: F) -> IC<T>
where
    C: Reflect,
    S: frunk::coproduct::CoprodInjector<C, Idx>,
    S: frunk::coproduct::CoproductFoldable<F, Flat<S, T>>,
    F: Clone,
{
    fn go<S, T, F>(folder: F, comp: S) -> Vec<T>
    where
        S: frunk::coproduct::CoproductFoldable<F, Flat<S, T>>,
        F: Clone,
    {
        match comp.fold(folder.clone()) {
            Flat::Continue(ic) => ic
                .components
                .into_iter()
                .flat_map(|c| go(folder.clone(), c))
                .collect(),
            Flat::Done(vs) => vs,
        }
    }

    IC {
        name: format!("{} ({})", chip.name(), label),
        intf: chip.reflect(),
        components: go(folder, S::inject(chip)),
    }
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek(), bi.peek()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, _) => return std::cmp::Ordering::Less,
            (_, None) => return std::cmp::Ordering::Greater,
            (Some(ac), Some(bc)) if ac.is_ascii_digit() && bc.is_ascii_digit() => {
                let an: u64 = ai
                    .by_ref()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .unwrap();
                let bn: u64 = bi
                    .by_ref()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .unwrap();
                let ord = an.cmp(&bn);
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
            }
            (Some(ac), Some(bc)) => {
                let ord = ac.cmp(bc);
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
                ai.next();
                bi.next();
            }
        }
    }
}

/// Show the connections of a chip after one level of expansion.
///
/// Each line is `source -> sink`. Chip inputs are sources; chip outputs are sinks.
/// Sub-component ports are labelled `{typename}{index}.{port}`.
///
/// ```ignore
/// let chip = And { a: Input1::new(), b: Input1::new(), out: Output::new() };
/// assert_eq!(print_graph(&chip), "And:\nnand0.a <- a\nnand0.b <- b\nnot1.a <- nand0.out\nout <- not1.out");
/// ```
///
/// Note: Claude has been given full latitude here as long as the output looks right,
/// and it's elected to sort strings at the end.
pub fn print_graph<C>(chip: &C) -> String
where
    C: Component + Reflect,
    C::Target: Component<Target = C::Target> + Reflect,
{
    let intf = chip.reflect();
    let subs = match chip.expand() {
        None => return format!("{}:\n  (primitive)", chip.name()),
        Some(s) => s,
    };
    print_ic_graph_named(&chip.name(), &intf, &subs.components)
}

/// Show the components making up this IC, with no additional expansion.
pub fn print_ic_graph<C>(ic: &IC<C>) -> String
where
    C: Reflect,
{
    print_ic_graph_named(&ic.name, &ic.intf, &ic.components)
}

fn print_ic_graph_named<C>(name: &str, intf: &Interface, components: &[C]) -> String
where
    C: Reflect,
{
    use crate::declare::BusRef;
    use std::collections::HashMap;

    let wire_id = |b: &BusRef| b.id.0;

    // wire_id -> Vec<(label, is_sink, offset, width, raw)>
    // raw=true: source label is printed as-is with no subscript (used for Const)
    let mut wires: HashMap<usize, Vec<(String, bool, usize, usize, bool)>> = HashMap::new();

    for (port, busref) in &intf.inputs {
        wires.entry(wire_id(busref)).or_default().push((
            port.clone(),
            false,
            busref.offset,
            busref.width,
            false,
        ));
    }
    for (port, busref) in &intf.outputs {
        wires.entry(wire_id(busref)).or_default().push((
            port.clone(),
            true,
            busref.offset,
            busref.width,
            false,
        ));
    }
    let mut index = 0usize;
    for sub in components.iter() {
        let sub_intf = sub.reflect();
        let label = format!("{}_{}", sub.name().to_lowercase(), index);
        for (port, busref) in &sub_intf.inputs {
            if let Some(value) = busref.fixed {
                // Fixed input: register the constant as a raw source on its own wire
                wires.entry(wire_id(busref)).or_default().push((
                    value.to_string(),
                    false,
                    0,
                    busref.width,
                    true,
                ));
            }
            wires.entry(wire_id(busref)).or_default().push((
                format!("{}.{}", label, port),
                true,
                busref.offset,
                busref.width,
                false,
            ));
        }
        for (port, busref) in &sub_intf.outputs {
            wires.entry(wire_id(busref)).or_default().push((
                format!("{}.{}", label, port),
                false,
                busref.offset,
                busref.width,
                false,
            ));
        }
        index += 1;
    }

    // Label for an endpoint: no subscript if the endpoint itself is 1-bit;
    // [lo] if 1 bit is selected from a wider bus; [lo..hi] for a full range.
    let ep_label = |name: &str, ep_w: usize, n: usize, lo: usize| -> String {
        if ep_w == 1 {
            name.to_string()
        } else if n == 1 {
            format!("{}[{}]", name, lo)
        } else {
            format!("{}[{}..{}]", name, lo, lo + n - 1)
        }
    };

    let mut lines: Vec<String> = wires
        .values()
        .flat_map(|endpoints| {
            let sources: Vec<_> = endpoints
                .iter()
                .filter(|(_, is_sink, _, _, _)| !is_sink)
                .collect();
            let sinks: Vec<_> = endpoints
                .iter()
                .filter(|(_, is_sink, _, _, _)| *is_sink)
                .collect();
            let mut result = vec![];
            for (src_name, _, src_off, src_w, src_raw) in &sources {
                for (sink_name, _, sink_off, sink_w, _) in &sinks {
                    let lo = (*src_off).max(*sink_off);
                    let hi = (src_off + src_w).min(sink_off + sink_w);
                    if lo >= hi {
                        continue;
                    }
                    let n = hi - lo;
                    let src_label = if *src_raw {
                        src_name.to_string()
                    } else {
                        ep_label(src_name, *src_w, n, lo)
                    };
                    result.push(format!(
                        "  {} <- {}",
                        ep_label(sink_name, *sink_w, n, lo),
                        src_label
                    ));
                }
            }
            result
        })
        .collect();

    // Sort by destination component index, then port name; chip outputs sort last.
    let sink_key = |line: &str| -> (usize, String) {
        let sink = line.split(" <- ").next().unwrap_or("");
        let sink = sink.split('[').next().unwrap_or(sink); // strip subscript before parsing
        if let Some(dot) = sink.find('.') {
            let comp = &sink[..dot];
            let num_start = comp.len()
                - comp
                    .chars()
                    .rev()
                    .take_while(|c| c.is_ascii_digit())
                    .count();
            let idx: usize = comp[num_start..].parse().unwrap_or(usize::MAX);
            (idx, sink[dot + 1..].to_string())
        } else {
            (usize::MAX, sink.to_string())
        }
    };
    lines.sort_by(|a, b| {
        let (ai, ap) = sink_key(a);
        let (bi, bp) = sink_key(b);
        ai.cmp(&bi).then_with(|| natural_cmp(&ap, &bp))
    });
    format!("{}:\n{}", name, lines.join("\n"))
}
